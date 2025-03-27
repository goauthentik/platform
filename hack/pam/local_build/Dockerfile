FROM ubuntu:24.04

COPY ./packages/pam_authentik_0.0.0~rc0_amd64.deb /tmp

RUN apt-get update && \
    apt-get install pamtester openssh-server nano -y && \
    dpkg -i /tmp/*deb && \
    rm /tmp/*deb && \
    echo "auth	required	pam_authentik.so url=http://10.120.20.68:9000 flow=default-authentication-flow" >> /etc/pam.d/sshd && \
    sed -i 's/KbdInteractiveAuthentication no/KbdInteractiveAuthentication yes/g' /etc/ssh/sshd_config
