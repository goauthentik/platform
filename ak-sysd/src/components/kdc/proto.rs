//! Just enough of a Kerberos KDC (RFC 4120) for Windows to log a local
//! account on through a `ksetup /mapuser`-mapped realm, so the credential
//! provider never needs the account's own password.
//!
//! Serves exactly two exchanges, AES256-CTS-HMAC-SHA1-96 only, no PAC:
//! - AS: a TGT for a user `arm`ed by a fresh authentik sign-in, requiring
//!   PA-ENC-TIMESTAMP and consuming the arming on success.
//! - TGS: a ticket for any service, encrypted with the machine key set via
//!   `ksetup /setcomputerpassword`, which LSA requests to verify the TGT.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use eyre::{Result, eyre};
use picky_asn1::bit_string::BitString;
use picky_asn1::restricted_string::Ia5String;
use picky_asn1::wrapper::{
    Asn1SequenceOf, BitStringAsn1, ExplicitContextTag0, ExplicitContextTag1, ExplicitContextTag2,
    ExplicitContextTag3, ExplicitContextTag4, ExplicitContextTag5, ExplicitContextTag6,
    ExplicitContextTag7, ExplicitContextTag9, ExplicitContextTag10, GeneralizedTimeAsn1,
    IntegerAsn1, OctetStringAsn1, Optional,
};
use picky_krb::constants::key_usages::{
    AS_REP_ENC, AS_REQ_TIMESTAMP, TGS_REP_ENC_SESSION_KEY, TGS_REP_ENC_SUB_KEY,
    TGS_REQ_PA_DATA_AP_REQ_AUTHENTICATOR, TICKET_REP,
};
use picky_krb::constants::types::{NT_PRINCIPAL, NT_SRV_INST};
use picky_krb::crypto::{Cipher, CipherSuite};
use picky_krb::data_types::{
    Authenticator, EncTicketPart, EncTicketPartInner, EncryptedData, EncryptionKey, EtypeInfo2,
    EtypeInfo2Entry, KerberosStringAsn1, KerberosTime, LastReqInner, PaData, PaEncTsEnc,
    PrincipalName, Ticket, TicketInner, TransitedEncoding,
};
use picky_krb::messages::{
    ApReq, AsRep, AsReq, EncAsRepPart, EncKdcRepPart, EncTgsRepPart, KdcRep, KrbError,
    KrbErrorInner, TgsRep, TgsReq,
};

const AES256: i32 = 18;
const PA_TGS_REQ: u8 = 1;
const PA_ENC_TIMESTAMP: u8 = 2;
const PA_ETYPE_INFO2: u8 = 19;

const KDC_ERR_C_PRINCIPAL_UNKNOWN: u32 = 6;
const KDC_ERR_ETYPE_NOSUPP: u32 = 14;
const KDC_ERR_PREAUTH_FAILED: u32 = 24;
const KDC_ERR_PREAUTH_REQUIRED: u32 = 25;
const KRB_AP_ERR_TKT_EXPIRED: u32 = 32;
const KRB_AP_ERR_SKEW: u32 = 37;
const KRB_AP_ERR_MODIFIED: u32 = 41;
const KRB_ERR_GENERIC: u32 = 60;

/// Ticket flags, numbered from the most significant bit (RFC 4120 5.3).
const FLAG_INITIAL: usize = 9;
const FLAG_PRE_AUTHENT: usize = 10;

const TICKET_LIFETIME: Duration = Duration::hours(10);
const MAX_SKEW: Duration = Duration::minutes(5);
/// How long a sign-in leaves its user able to get a TGT: Windows submits the
/// logon a moment after `Connect` returns, so this only has to cover that.
pub const ARM_WINDOW: Duration = Duration::minutes(2);

/// A user cleared for exactly one AS exchange.
struct Armed {
    password: String,
    until: DateTime<Utc>,
}

pub struct Kdc {
    pub realm: String,
    /// Also used by the TGS exchange to derive per-service keys, so it has to
    /// be the password `ksetup /setcomputerpassword` was given.
    machine_password: String,
    /// Random per process: a restart only invalidates outstanding TGTs, which
    /// Windows renews from the cached password anyway.
    krbtgt_key: Vec<u8>,
    armed: Mutex<HashMap<String, Armed>>,
}

/// Turns an error code into a reply as soon as it is known, so handlers can `?`.
struct KrbErr(u32);

impl From<eyre::Report> for KrbErr {
    fn from(e: eyre::Report) -> Self {
        tracing::debug!("kdc: malformed request: {e:?}");
        KrbErr(KRB_ERR_GENERIC)
    }
}

impl Kdc {
    pub fn new(realm: String, machine_password: String) -> Self {
        Self {
            realm,
            machine_password,
            krbtgt_key: random_key(),
            armed: Mutex::new(HashMap::new()),
        }
    }

    /// Lets `username` get one TGT with `password` until [`ARM_WINDOW`] runs out.
    pub fn arm(&self, username: &str, password: &str, now: DateTime<Utc>) {
        let mut armed = self.armed.lock().unwrap_or_else(|e| e.into_inner());
        armed.retain(|_, a| a.until > now);
        armed.insert(
            username.to_lowercase(),
            Armed {
                password: password.to_string(),
                until: now + ARM_WINDOW,
            },
        );
    }

    /// One request in, one reply out: AS-REP, TGS-REP or KRB-ERROR.
    pub fn handle(&self, req: &[u8], now: DateTime<Utc>) -> Vec<u8> {
        // [APPLICATION 10] and [APPLICATION 12], constructed.
        let reply = match req.first() {
            Some(0x6a) => self.as_exchange(req, now),
            Some(0x6c) => self.tgs_exchange(req, now),
            _ => Err((KrbErr(KRB_ERR_GENERIC), None)),
        };
        match reply {
            Ok(reply) => reply,
            Err((KrbErr(code), e_data)) => {
                tracing::info!("kdc: replying with error {code}");
                self.error(code, e_data, now)
            }
        }
    }

    fn as_exchange(
        &self,
        req: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Vec<u8>, (KrbErr, Option<Vec<u8>>)> {
        let req: AsReq = der(req).map_err(|e| (e.into(), None))?;
        let req = &req.0;
        let body = &req.req_body.0;
        let cname = body
            .cname
            .0
            .as_ref()
            .ok_or((KrbErr(KRB_ERR_GENERIC), None))?;
        // `alice@REALM` arrives as one NT-ENTERPRISE component.
        let username = principal_components(&cname.0)
            .first()
            .and_then(|c| c.split('@').next())
            .map(str::to_lowercase)
            .ok_or((KrbErr(KDC_ERR_C_PRINCIPAL_UNKNOWN), None))?;
        if !offers_aes256(&body.etype.0) {
            return Err((KrbErr(KDC_ERR_ETYPE_NOSUPP), None));
        }

        let password = {
            let armed = self.armed.lock().unwrap_or_else(|e| e.into_inner());
            match armed.get(&username) {
                Some(a) if a.until > now => a.password.clone(),
                _ => {
                    tracing::info!(username, "kdc: AS-REQ for a user with no fresh sign-in");
                    return Err((KrbErr(KDC_ERR_C_PRINCIPAL_UNKNOWN), None));
                }
            }
        };
        let salt = format!("{}{}", self.realm, username);
        let user_key = aes()
            .generate_key_from_password(password.as_bytes(), salt.as_bytes())
            .map_err(|_| (KrbErr(KRB_ERR_GENERIC), None))?;

        let Some(timestamp) = padata(&req.padata.0, PA_ENC_TIMESTAMP) else {
            return Err((
                KrbErr(KDC_ERR_PREAUTH_REQUIRED),
                Some(self.preauth_hint(&salt)?),
            ));
        };
        let ts: EncryptedData = der(&timestamp).map_err(|e| (e.into(), None))?;
        let ts: PaEncTsEnc = aes()
            .decrypt(&user_key, AS_REQ_TIMESTAMP, &ts.cipher.0.0)
            .ok()
            .and_then(|plain| der(&plain).ok())
            .ok_or((KrbErr(KDC_ERR_PREAUTH_FAILED), None))?;
        let client_time: DateTime<Utc> = ts.patimestamp.0.0.into();
        if (client_time - now).abs() > MAX_SKEW {
            return Err((KrbErr(KRB_AP_ERR_SKEW), None));
        }

        // Single use: a second logon needs a second sign-in.
        self.armed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&username);
        tracing::info!(username, "kdc: issuing TGT");

        let mut flags = BitString::with_len(32);
        flags.set(FLAG_INITIAL, true);
        flags.set(FLAG_PRE_AUTHENT, true);
        let cname = principal(NT_PRINCIPAL, &[&username]);
        let sname = principal(NT_SRV_INST, &["krbtgt", &self.realm]);
        let issued = Issue {
            flags,
            cname,
            sname,
            auth_time: now,
            end_time: now + TICKET_LIFETIME,
        };
        let (ticket, session_key) = self
            .ticket(&issued, &self.krbtgt_key)
            .map_err(|e| (e.into(), None))?;
        let enc_part = EncAsRepPart::from(self.enc_rep_part(&issued, session_key, &body.nonce.0));
        let enc_part = encrypt(&user_key, AS_REP_ENC, &enc_part).map_err(|e| (e.into(), None))?;
        to_der(&AsRep::from(self.kdc_rep(11, &issued, ticket, enc_part)))
            .map_err(|e| (e.into(), None))
    }

    fn tgs_exchange(
        &self,
        req: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Vec<u8>, (KrbErr, Option<Vec<u8>>)> {
        let req: TgsReq = der(req).map_err(|e| (e.into(), None))?;
        let req = &req.0;
        let body = &req.req_body.0;
        let sname = body
            .sname
            .0
            .as_ref()
            .ok_or((KrbErr(KRB_ERR_GENERIC), None))?
            .0
            .clone();
        if !offers_aes256(&body.etype.0) {
            return Err((KrbErr(KDC_ERR_ETYPE_NOSUPP), None));
        }

        let ap_req = padata(&req.padata.0, PA_TGS_REQ).ok_or((KrbErr(KRB_ERR_GENERIC), None))?;
        let ap_req: ApReq = der(&ap_req).map_err(|e| (e.into(), None))?;
        let tgt: EncTicketPart = aes()
            .decrypt(
                &self.krbtgt_key,
                TICKET_REP,
                &ap_req.0.ticket.0.0.enc_part.0.cipher.0.0,
            )
            .ok()
            .and_then(|plain| der(&plain).ok())
            .ok_or((KrbErr(KRB_AP_ERR_MODIFIED), None))?;
        let tgt = tgt.0;
        let end_time: DateTime<Utc> = tgt.endtime.0.0.clone().into();
        if end_time < now {
            return Err((KrbErr(KRB_AP_ERR_TKT_EXPIRED), None));
        }

        let tgt_key = tgt.key.0.key_value.0.0.clone();
        let authenticator: Authenticator = aes()
            .decrypt(
                &tgt_key,
                TGS_REQ_PA_DATA_AP_REQ_AUTHENTICATOR,
                &ap_req.0.authenticator.0.cipher.0.0,
            )
            .ok()
            .and_then(|plain| der(&plain).ok())
            .ok_or((KrbErr(KRB_AP_ERR_MODIFIED), None))?;
        let authenticator = authenticator.0;
        if authenticator.cname.0 != tgt.cname.0 {
            return Err((KrbErr(KRB_AP_ERR_MODIFIED), None));
        }
        let client_time: DateTime<Utc> = authenticator.ctime.0.0.clone().into();
        if (client_time - now).abs() > MAX_SKEW {
            return Err((KrbErr(KRB_AP_ERR_SKEW), None));
        }
        // ponytail: the authenticator's checksum over req-body is not checked,
        // which matters only to someone already able to sniff loopback; verify
        // it against the raw body bytes if this ever listens off-box.

        // Windows decrypts with its machine password salted like an MIT
        // principal of the name it asked for, so derive the key the same way.
        let salt = format!("{}{}", self.realm, principal_components(&sname).concat());
        let service_key = aes()
            .generate_key_from_password(self.machine_password.as_bytes(), salt.as_bytes())
            .map_err(|_| (KrbErr(KRB_ERR_GENERIC), None))?;
        tracing::info!(
            sname = principal_components(&sname).join("/"),
            "kdc: issuing service ticket"
        );

        let mut flags = tgt.flags.0.0.clone();
        flags.set(FLAG_INITIAL, false);
        let issued = Issue {
            flags,
            cname: tgt.cname.0.clone(),
            sname,
            auth_time: tgt.auth_time.0.0.clone().into(),
            end_time,
        };
        let (ticket, session_key) = self
            .ticket(&issued, &service_key)
            .map_err(|e| (e.into(), None))?;
        let enc_part = EncTgsRepPart::from(self.enc_rep_part(&issued, session_key, &body.nonce.0));
        let enc_part = match authenticator.subkey.0 {
            Some(subkey) => encrypt(&subkey.0.key_value.0.0, TGS_REP_ENC_SUB_KEY, &enc_part),
            None => encrypt(&tgt_key, TGS_REP_ENC_SESSION_KEY, &enc_part),
        }
        .map_err(|e| (e.into(), None))?;
        to_der(&TgsRep::from(self.kdc_rep(13, &issued, ticket, enc_part)))
            .map_err(|e| (e.into(), None))
    }

    /// METHOD-DATA for KDC_ERR_PREAUTH_REQUIRED: which etype and salt to use.
    fn preauth_hint(&self, salt: &str) -> Result<Vec<u8>, (KrbErr, Option<Vec<u8>>)> {
        let etype_info: EtypeInfo2 = Asn1SequenceOf::from(vec![EtypeInfo2Entry {
            etype: ExplicitContextTag0::from(int(AES256)),
            salt: Optional::from(Some(ExplicitContextTag1::from(kstring(salt)))),
            s2kparams: Optional::from(None),
        }]);
        let etype_info = to_der(&etype_info).map_err(|e| (e.into(), None))?;
        to_der(&Asn1SequenceOf::from(vec![
            pa(PA_ETYPE_INFO2, etype_info),
            pa(PA_ENC_TIMESTAMP, vec![]),
        ]))
        .map_err(|e| (e.into(), None))
    }

    /// A ticket for `issued`, sealed with `key`, and its fresh session key.
    fn ticket(&self, issued: &Issue, key: &[u8]) -> Result<(Ticket, Vec<u8>)> {
        let session_key = random_key();
        let part = EncTicketPart::from(EncTicketPartInner {
            flags: ExplicitContextTag0::from(BitStringAsn1::from(issued.flags.clone())),
            key: ExplicitContextTag1::from(encryption_key(&session_key)),
            crealm: ExplicitContextTag2::from(kstring(&self.realm)),
            cname: ExplicitContextTag3::from(issued.cname.clone()),
            transited: ExplicitContextTag4::from(TransitedEncoding {
                tr_type: ExplicitContextTag0::from(int(1)),
                contents: ExplicitContextTag1::from(OctetStringAsn1::from(vec![])),
            }),
            auth_time: ExplicitContextTag5::from(time(issued.auth_time)),
            starttime: Optional::from(None),
            endtime: ExplicitContextTag7::from(time(issued.end_time)),
            renew_till: Optional::from(None),
            caddr: Optional::from(None),
            authorization_data: Optional::from(None),
        });
        let ticket = Ticket::from(TicketInner {
            tkt_vno: ExplicitContextTag0::from(int(5)),
            realm: ExplicitContextTag1::from(kstring(&self.realm)),
            sname: ExplicitContextTag2::from(issued.sname.clone()),
            enc_part: ExplicitContextTag3::from(encrypt(key, TICKET_REP, &part)?),
        });
        Ok((ticket, session_key))
    }

    fn enc_rep_part(
        &self,
        issued: &Issue,
        session_key: Vec<u8>,
        nonce: &IntegerAsn1,
    ) -> EncKdcRepPart {
        EncKdcRepPart {
            key: ExplicitContextTag0::from(encryption_key(&session_key)),
            last_req: ExplicitContextTag1::from(Asn1SequenceOf::from(vec![LastReqInner {
                lr_type: ExplicitContextTag0::from(int(0)),
                lr_value: ExplicitContextTag1::from(time(issued.auth_time)),
            }])),
            nonce: ExplicitContextTag2::from(nonce.clone()),
            key_expiration: Optional::from(None),
            flags: ExplicitContextTag4::from(BitStringAsn1::from(issued.flags.clone())),
            auth_time: ExplicitContextTag5::from(time(issued.auth_time)),
            start_time: Optional::from(None),
            end_time: ExplicitContextTag7::from(time(issued.end_time)),
            renew_till: Optional::from(None),
            srealm: ExplicitContextTag9::from(kstring(&self.realm)),
            sname: ExplicitContextTag10::from(issued.sname.clone()),
            caddr: Optional::from(None),
            encrypted_pa_data: Optional::from(None),
        }
    }

    fn kdc_rep(
        &self,
        msg_type: i32,
        issued: &Issue,
        ticket: Ticket,
        enc_part: EncryptedData,
    ) -> KdcRep {
        KdcRep {
            pvno: ExplicitContextTag0::from(int(5)),
            msg_type: ExplicitContextTag1::from(int(msg_type)),
            padata: Optional::from(None),
            crealm: ExplicitContextTag3::from(kstring(&self.realm)),
            cname: ExplicitContextTag4::from(issued.cname.clone()),
            ticket: ExplicitContextTag5::from(ticket),
            enc_part: ExplicitContextTag6::from(enc_part),
        }
    }

    fn error(&self, code: u32, e_data: Option<Vec<u8>>, now: DateTime<Utc>) -> Vec<u8> {
        let err = KrbError::from(KrbErrorInner {
            pvno: ExplicitContextTag0::from(int(5)),
            msg_type: ExplicitContextTag1::from(int(30)),
            ctime: Optional::from(None),
            cusec: Optional::from(None),
            stime: ExplicitContextTag4::from(time(now)),
            susec: ExplicitContextTag5::from(int(0)),
            error_code: ExplicitContextTag6::from(code),
            crealm: Optional::from(None),
            cname: Optional::from(None),
            realm: ExplicitContextTag9::from(kstring(&self.realm)),
            sname: ExplicitContextTag10::from(principal(NT_SRV_INST, &["krbtgt", &self.realm])),
            e_text: Optional::from(None),
            e_data: Optional::from(e_data.map(|d| {
                picky_asn1::wrapper::ExplicitContextTag12::from(OctetStringAsn1::from(d))
            })),
        });
        // Every field above is fixed or already validated; nothing to fail on.
        to_der(&err).unwrap_or_default()
    }
}

/// What a ticket and its reply part both say.
struct Issue {
    flags: BitString,
    cname: PrincipalName,
    sname: PrincipalName,
    auth_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

fn aes() -> Box<dyn Cipher> {
    CipherSuite::Aes256CtsHmacSha196.cipher()
}

fn random_key() -> Vec<u8> {
    use rand::Rng;
    let mut key = vec![0u8; 32];
    rand::rng().fill_bytes(&mut key);
    key
}

fn der<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    picky_asn1_der::from_bytes(bytes).map_err(|e| eyre!("{e}"))
}

fn to_der<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    picky_asn1_der::to_vec(value).map_err(|e| eyre!("{e}"))
}

fn encrypt<T: serde::Serialize>(key: &[u8], usage: i32, value: &T) -> Result<EncryptedData> {
    let cipher = aes()
        .encrypt(key, usage, &to_der(value)?)
        .map_err(|e| eyre!("{e}"))?;
    Ok(EncryptedData {
        etype: ExplicitContextTag0::from(int(AES256)),
        kvno: Optional::from(None),
        cipher: ExplicitContextTag2::from(OctetStringAsn1::from(cipher)),
    })
}

fn int(v: i32) -> IntegerAsn1 {
    IntegerAsn1::from_bytes_be_signed(v.to_be_bytes().to_vec())
}

fn int_value(v: &IntegerAsn1) -> i64 {
    let bytes = v.as_signed_bytes_be();
    let fill = if bytes.first().is_some_and(|b| b & 0x80 != 0) {
        0xff
    } else {
        0
    };
    let mut buf = [fill; 8];
    let n = bytes.len().min(8);
    buf[8 - n..].copy_from_slice(&bytes[bytes.len() - n..]);
    i64::from_be_bytes(buf)
}

fn offers_aes256(etypes: &Asn1SequenceOf<IntegerAsn1>) -> bool {
    etypes.0.iter().any(|e| int_value(e) == AES256 as i64)
}

fn kstring(s: &str) -> KerberosStringAsn1 {
    // Realm and principal names here are ASCII; anything else becomes empty
    // and fails to match rather than panicking.
    KerberosStringAsn1::from(Ia5String::from_string(s.to_string()).unwrap_or_default())
}

fn time(t: DateTime<Utc>) -> KerberosTime {
    GeneralizedTimeAsn1::from(picky_asn1::date::GeneralizedTime::from(t))
}

fn principal(name_type: u8, components: &[&str]) -> PrincipalName {
    PrincipalName {
        name_type: ExplicitContextTag0::from(int(name_type.into())),
        name_string: ExplicitContextTag1::from(Asn1SequenceOf::from(
            components.iter().map(|c| kstring(c)).collect::<Vec<_>>(),
        )),
    }
}

fn principal_components(p: &PrincipalName) -> Vec<String> {
    p.name_string
        .0
        .0
        .iter()
        .map(|s| s.0.as_utf8().to_string())
        .collect()
}

fn encryption_key(key: &[u8]) -> EncryptionKey {
    EncryptionKey {
        key_type: ExplicitContextTag0::from(int(AES256)),
        key_value: ExplicitContextTag1::from(OctetStringAsn1::from(key.to_vec())),
    }
}

fn pa(padata_type: u8, data: Vec<u8>) -> PaData {
    PaData {
        padata_type: ExplicitContextTag1::from(int(padata_type.into())),
        padata_data: ExplicitContextTag2::from(OctetStringAsn1::from(data)),
    }
}

fn padata(
    padata: &Option<ExplicitContextTag3<Asn1SequenceOf<PaData>>>,
    padata_type: u8,
) -> Option<Vec<u8>> {
    padata
        .as_ref()?
        .0
        .0
        .iter()
        .find(|p| int_value(&p.padata_type.0) == padata_type as i64)
        .map(|p| p.padata_data.0.0.clone())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use picky_asn1::wrapper::ExplicitContextTag8;
    use picky_krb::data_types::{ApOptions, AuthenticatorInner};
    use picky_krb::messages::{ApReqInner, KdcReq, KdcReqBody};

    const REALM: &str = "AUTHENTIK.LOCAL";
    const MACHINE_PASSWORD: &str = "machine-password";

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn body(cname: Option<PrincipalName>, sname: PrincipalName) -> KdcReqBody {
        KdcReqBody {
            kdc_options: ExplicitContextTag0::from(BitStringAsn1::from(BitString::with_len(32))),
            cname: Optional::from(cname.map(ExplicitContextTag1::from)),
            realm: ExplicitContextTag2::from(kstring(REALM)),
            sname: Optional::from(Some(ExplicitContextTag3::from(sname))),
            from: Optional::from(None),
            till: ExplicitContextTag5::from(time(now() + Duration::days(1))),
            rtime: Optional::from(None),
            nonce: ExplicitContextTag7::from(int(1234)),
            etype: ExplicitContextTag8::from(Asn1SequenceOf::from(vec![int(23), int(AES256)])),
            addresses: Optional::from(None),
            enc_authorization_data: Optional::from(None),
            additional_tickets: Optional::from(None),
        }
    }

    fn as_req(username: &str, password: Option<&str>) -> Vec<u8> {
        let padata = password.map(|password| {
            let key = aes()
                .generate_key_from_password(
                    password.as_bytes(),
                    format!("{REALM}{username}").as_bytes(),
                )
                .unwrap();
            let ts = PaEncTsEnc {
                patimestamp: ExplicitContextTag0::from(time(now())),
                pausec: Optional::from(None),
            };
            let ts = encrypt(&key, AS_REQ_TIMESTAMP, &ts).unwrap();
            ExplicitContextTag3::from(Asn1SequenceOf::from(vec![pa(
                PA_ENC_TIMESTAMP,
                to_der(&ts).unwrap(),
            )]))
        });
        to_der(&AsReq::from(KdcReq {
            pvno: ExplicitContextTag1::from(int(5)),
            msg_type: ExplicitContextTag2::from(int(10)),
            padata: Optional::from(padata),
            req_body: ExplicitContextTag4::from(body(
                Some(principal(NT_PRINCIPAL, &[username])),
                principal(NT_SRV_INST, &["krbtgt", REALM]),
            )),
        }))
        .unwrap()
    }

    fn error_code(reply: &[u8]) -> u32 {
        let err: KrbError = der(reply).unwrap();
        err.0.error_code.0
    }

    /// Logs `username` in, returning the TGT and its session key as Windows
    /// would see them.
    fn get_tgt(kdc: &Kdc, username: &str, password: &str) -> (Ticket, Vec<u8>) {
        let reply = kdc.handle(&as_req(username, Some(password)), now());
        let rep: AsRep = der(&reply).unwrap();
        let key = aes()
            .generate_key_from_password(
                password.as_bytes(),
                format!("{REALM}{username}").as_bytes(),
            )
            .unwrap();
        let part: EncAsRepPart = der(&aes()
            .decrypt(&key, AS_REP_ENC, &rep.0.enc_part.0.cipher.0.0)
            .unwrap())
        .unwrap();
        assert_eq!(int_value(&part.0.nonce.0), 1234);
        (rep.0.ticket.0, part.0.key.0.key_value.0.0)
    }

    fn tgs_req(tgt: Ticket, session_key: &[u8], username: &str, sname: &[&str]) -> Vec<u8> {
        let authenticator = Authenticator::from(AuthenticatorInner {
            authenticator_vno: ExplicitContextTag0::from(int(5)),
            crealm: ExplicitContextTag1::from(kstring(REALM)),
            cname: ExplicitContextTag2::from(principal(NT_PRINCIPAL, &[username])),
            cksum: Optional::from(None),
            cusec: ExplicitContextTag4::from(int(0)),
            ctime: ExplicitContextTag5::from(time(now())),
            subkey: Optional::from(None),
            seq_number: Optional::from(None),
            authorization_data: Optional::from(None),
        });
        let ap_req = ApReq::from(ApReqInner {
            pvno: ExplicitContextTag0::from(int(5)),
            msg_type: ExplicitContextTag1::from(int(14)),
            ap_options: ExplicitContextTag2::from(ApOptions::from(BitString::with_len(32))),
            ticket: ExplicitContextTag3::from(tgt),
            authenticator: ExplicitContextTag4::from(
                encrypt(
                    session_key,
                    TGS_REQ_PA_DATA_AP_REQ_AUTHENTICATOR,
                    &authenticator,
                )
                .unwrap(),
            ),
        });
        to_der(&TgsReq::from(KdcReq {
            pvno: ExplicitContextTag1::from(int(5)),
            msg_type: ExplicitContextTag2::from(int(12)),
            padata: Optional::from(Some(ExplicitContextTag3::from(Asn1SequenceOf::from(vec![
                pa(PA_TGS_REQ, to_der(&ap_req).unwrap()),
            ])))),
            req_body: ExplicitContextTag4::from(body(None, principal(NT_SRV_INST, sname))),
        }))
        .unwrap()
    }

    #[test]
    fn full_logon_yields_a_ticket_the_machine_can_read() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        kdc.arm("Alice", "user-password", now());

        // Windows always tries without pre-auth first.
        assert_eq!(
            error_code(&kdc.handle(&as_req("alice", None), now())),
            KDC_ERR_PREAUTH_REQUIRED
        );
        let (tgt, tgt_key) = get_tgt(&kdc, "alice", "user-password");

        let reply = kdc.handle(
            &tgs_req(tgt, &tgt_key, "alice", &["host", "desktop-1"]),
            now(),
        );
        let rep: TgsRep = der(&reply).unwrap();
        let machine_key = aes()
            .generate_key_from_password(
                MACHINE_PASSWORD.as_bytes(),
                format!("{REALM}hostdesktop-1").as_bytes(),
            )
            .unwrap();
        let ticket: EncTicketPart = der(&aes()
            .decrypt(
                &machine_key,
                TICKET_REP,
                &rep.0.ticket.0.0.enc_part.0.cipher.0.0,
            )
            .unwrap())
        .unwrap();
        assert_eq!(principal_components(&ticket.0.cname.0), vec!["alice"]);
        assert_eq!(ticket.0.crealm.0.0.as_utf8(), REALM);
    }

    #[test]
    fn a_user_without_a_fresh_sign_in_gets_nothing() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        assert_eq!(
            error_code(&kdc.handle(&as_req("alice", Some("pw")), now())),
            KDC_ERR_C_PRINCIPAL_UNKNOWN
        );

        kdc.arm("alice", "pw", now() - ARM_WINDOW - Duration::seconds(1));
        assert_eq!(
            error_code(&kdc.handle(&as_req("alice", Some("pw")), now())),
            KDC_ERR_C_PRINCIPAL_UNKNOWN,
            "an arming past its window must not count"
        );
    }

    #[test]
    fn a_sign_in_is_good_for_one_tgt() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        kdc.arm("alice", "pw", now());
        get_tgt(&kdc, "alice", "pw");
        assert_eq!(
            error_code(&kdc.handle(&as_req("alice", Some("pw")), now())),
            KDC_ERR_C_PRINCIPAL_UNKNOWN
        );
    }

    #[test]
    fn a_wrong_password_fails_preauth_and_keeps_the_arming() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        kdc.arm("alice", "pw", now());
        assert_eq!(
            error_code(&kdc.handle(&as_req("alice", Some("wrong")), now())),
            KDC_ERR_PREAUTH_FAILED
        );
        get_tgt(&kdc, "alice", "pw");
    }

    #[test]
    fn a_forged_tgt_is_rejected() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        let other = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        other.arm("alice", "pw", now());
        let (tgt, key) = get_tgt(&other, "alice", "pw");
        assert_eq!(
            error_code(&kdc.handle(&tgs_req(tgt, &key, "alice", &["host", "x"]), now())),
            KRB_AP_ERR_MODIFIED
        );
    }

    #[test]
    fn garbage_gets_an_error_not_a_panic() {
        let kdc = Kdc::new(REALM.to_string(), MACHINE_PASSWORD.to_string());
        assert_eq!(
            error_code(&kdc.handle(&[0x6a, 0x01, 0xff], now())),
            KRB_ERR_GENERIC
        );
        assert_eq!(error_code(&kdc.handle(&[], now())), KRB_ERR_GENERIC);
    }
}
