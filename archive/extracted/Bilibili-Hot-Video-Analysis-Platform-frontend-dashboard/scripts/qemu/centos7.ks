install
cdrom
lang en_US.UTF-8
keyboard us
timezone Asia/Shanghai --isUtc

rootpw --plaintext hadoop123
user --name=hadoop --password=hadoop123 --plaintext --groups=wheel

network --bootproto=dhcp --device=eth0 --onboot=on --activate
network --hostname=hadoop-base

firewall --disabled
selinux --disabled
firstboot --disable

bootloader --location=mbr
zerombr
clearpart --all --initlabel
autopart --type=lvm

services --enabled=sshd,NetworkManager --disabled=firewalld
shutdown

%packages --ignoremissing
@core
openssh-server
openssh-clients
java-1.8.0-openjdk
java-1.8.0-openjdk-devel
net-tools
vim
ntp
%end

%post --log=/root/ks-post.log
systemctl enable sshd
systemctl enable NetworkManager

sed -i 's/^#\?PasswordAuthentication .*/PasswordAuthentication yes/' /etc/ssh/sshd_config
sed -i 's/^#\?UseDNS .*/UseDNS no/' /etc/ssh/sshd_config || echo 'UseDNS no' >> /etc/ssh/sshd_config

cat > /etc/profile.d/java.sh << 'EOF'
export JAVA_HOME=/usr/lib/jvm/java-1.8.0-openjdk
export PATH=$JAVA_HOME/bin:$PATH
EOF

cat >> /etc/hosts << 'EOF'
192.168.100.10 master
192.168.100.11 worker1
192.168.100.12 worker2
EOF

cat >> /etc/rc.local << 'EOF'
echo never > /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null || true
echo never > /sys/kernel/mm/transparent_hugepage/defrag 2>/dev/null || true
EOF
chmod +x /etc/rc.local

mkdir -p /home/hadoop/.ssh
chmod 700 /home/hadoop/.ssh
chown -R hadoop:hadoop /home/hadoop/.ssh
%end
