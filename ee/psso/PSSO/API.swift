import AuthenticationServices
import CryptoKit
import Foundation
import Generated
import OSLog

class API {

    public static var shared: API = API()

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "API")

    func RegisterDevice(loginManager: ASAuthorizationProviderExtensionLoginManager) async
        -> ASAuthorizationProviderExtensionLoginConfiguration?
    {
        do {
            let (SignKeyID, DeviceSigningKey, _) = try getPublicKeyString(
                from: loginManager.key(for: .currentDeviceSigning)!)!
            let (EncKeyID, DeviceEncryptionKey, _) = try getPublicKeyString(
                from: loginManager.key(for: .currentDeviceEncryption)!)!
            self.logger.debug("registering device with sysd...")
            let config = try await SysdBridge.shared.pssoRegisterDevice(
                deviceSigningKey: DeviceSigningKey,
                deviceEncryptionKey: DeviceEncryptionKey,
                encKeyID: EncKeyID,
                signKeyID: SignKeyID
            )
            return config
        } catch {
            self.logger.error("failed to register: \(error)")
            return nil
        }
    }

    func RegisterUser(
        loginConfig: ASAuthorizationProviderExtensionUserLoginConfiguration,
        loginManger: ASAuthorizationProviderExtensionLoginManager,
    ) async -> ASAuthorizationProviderExtensionRegistrationResult {
        do {
            let (EnclaveKeyID, UserSecureEnclaveKey, _) = try getPublicKeyString(
                from: loginManger.key(for: .userSecureEnclaveKey)!)!
            self.logger.debug("registering user with sysd...")
            let registerResult = try await SysdBridge.shared
                .pssoRegisterUser(
                    enclaveKeyID: EnclaveKeyID,
                    userSecureEnclaveKey: UserSecureEnclaveKey
                )
            loginConfig.loginUserName = registerResult
            try loginManger.saveUserLoginConfiguration(loginConfig)
            return .success
        } catch {
            self.logger.error("failed to register: \(error)")
            return .failed
        }
    }

    func getPublicKey(from privateKey: SecKey) -> SecKey? {
        // Use SecKeyCopyPublicKey to get the public key from the private key
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            NSLog("Error: Could not get public key from private key")
            return nil
        }

        return publicKey
    }

    // Function to compute the SHA-256 hash of the public key data and return it as a base64 string
    func getKeyID(from publicKeyDERData: Data) throws -> String {
        let hash = SHA256.hash(data: publicKeyDERData)
        return Data(hash).base64EncodedString()
    }

    // Function to build an ASN.1 header based on the key type (RSA, EC)
    func addX509Header(to publicKeyDERData: Data, keyType: CFString) -> Data {
        var header: [UInt8] = []

        if keyType == kSecAttrKeyTypeRSA {
            // Header for RSA (OID: 1.2.840.113549.1.1.1)
            header = [
                0x30, 0x82, 0x01, 0x22,  // SEQUENCE (SubjectPublicKeyInfo)
                0x30, 0x0D,  // SEQUENCE (AlgorithmIdentifier)
                0x06, 0x09,  // OBJECT IDENTIFIER (1.2.840.113549.1.1.1 -> rsaEncryption)
                0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01,
                0x05, 0x00,  // NULL (Parameters)
                0x03, 0x82, 0x01, 0x0F,  // BIT STRING
                0x00,  // Unused bits indicator for BIT STRING
            ]
        } else if keyType == kSecAttrKeyTypeEC {
            // Header for EC (OID: 1.2.840.10045.2.1 for ecPublicKey with secp256r1 curve OID: 1.2.840.10045.3.1.7)
            header = [
                0x30, 0x59,  // SEQUENCE (SubjectPublicKeyInfo)
                0x30, 0x13,  // SEQUENCE (AlgorithmIdentifier)
                0x06, 0x07,  // OBJECT IDENTIFIER (1.2.840.10045.2.1 -> ecPublicKey)
                0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01,
                0x06, 0x08,  // OBJECT IDENTIFIER (1.2.840.10045.3.1.7 -> secp256r1)
                0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07,
                0x03, 0x42,  // BIT STRING
                0x00,  // Unused bits indicator for BIT STRING
            ]
        }

        // Add the header to the public key data
        var x509PublicKey = Data(header)
        x509PublicKey.append(publicKeyDERData)

        return x509PublicKey
    }

    // Function to extract the public key as PEM format and compute the Key ID
    func getPublicKeyString(from privateKey: SecKey) throws -> (String, String, Data)? {
        // Get the public key from the private key
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            return nil
        }

        // Determine the type of the key (RSA, EC, etc.)
        let attributes = SecKeyCopyAttributes(publicKey) as! [CFString: Any]
        let keyType = attributes[kSecAttrKeyType] as! CFString

        // Extract public key data in DER format
        var error: Unmanaged<CFError>?
        guard let publicKeyData = SecKeyCopyExternalRepresentation(publicKey, &error) else {
            if let cfError = error?.takeRetainedValue() {
                throw cfError as Error
            }
            return nil
        }

        let publicKeyDERData = publicKeyData as Data

        // Add the X.509 header to the raw public key data based on its type (RSA or EC)
        let x509PublicKeyData = addX509Header(to: publicKeyDERData, keyType: keyType)

        // Convert X.509 DER data to base64-encoded PEM format
        let publicKeyString = x509PublicKeyData.base64EncodedString(options: [
            .lineLength64Characters
        ])

        // Wrap the base64 encoded string with PEM headers
        let publicKeyPEM = """
            -----BEGIN PUBLIC KEY-----
            \(publicKeyString)
            -----END PUBLIC KEY-----
            """

        // Compute Key ID (SHA-256 hash of the X.509 public key data)
        let keyID = try getKeyID(from: publicKeyDERData)

        // Return both keyID, PEM encoded public key, and X.509 DER format
        return (keyID, publicKeyPEM, x509PublicKeyData)
    }
}
