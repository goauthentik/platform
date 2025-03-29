FROM ubuntu:24.04

RUN apt-get update && \
    apt-get install pamtester openssh-server nano htop -y && \
    sed -i 's/KbdInteractiveAuthentication no/KbdInteractiveAuthentication yes/g' /etc/ssh/sshd_config && \
    mkdir /run/sshd && \
    adduser akadmin && \
    passwd -u akadmin

CMD ["/usr/sbin/sshd", "-D", "-e"]
