#Prepare base env
sudo apt update
sudo apt upgrade -y
sudo apt install gcc -y
sudo apt install libfontconfig -y
sudo apt install libfontconfig1-dev -y
sudo apt install dos2unix -y
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo apt install unzip -y
sudo apt install git -y
#sudo apt install bind9 -y
#echo "net.ipv4.tcp_fastopen = 3" >> /etc/sysctl.conf
#sysctl -p
sudo curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update
#Create swap file
sudo swapoff /swapfile
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
sudo free -h
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Start install ppaass
# sudo ps -ef | grep ppaass-v3-proxy | grep -v grep | awk '{print $2}' | xargs sudo kill

sudo rm -rf /ppaass-v3/build
sudo rm -rf /ppaass-v3/sourcecode
# Build
sudo mkdir /ppaass-v3
sudo mkdir /ppaass-v3/sourcecode
sudo mkdir /ppaass-v3/build
sudo mkdir /ppaass-v3/build/resources
sudo mkdir /ppaass-v3/build/resources/agent_user
sudo mkdir /ppaass-v3/build/resources/forward_user
# Pull ppaass
cd /ppaass-v3/sourcecode
sudo git clone -b main https://github.com/quhxuxm/ppaass-v3.git ppaass-v3
sudo chmod 777 ppaass-v3
cd /ppaass-v3/sourcecode/ppaass-v3
sudo git pull
cd /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-cli
cargo build --release
cd /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-tool
cargo build --release

# ps -ef | grep gradle | grep -v grep | awk '{print $2}' | xargs kill -9
sudo cp -r /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-cli/resources/* /ppaass-v3/build/resources
sudo cp -r /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-cli/resources/agent_user/* /ppaass-v3/build/resources/agent_user
sudo cp -r /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-cli/resources/forward_user/* /ppaass-v3/build/resources/forward_user
sudo cp /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-cli/target/release/ppaass-v3-proxy-cli /ppaass-v3/build/ppaass-v3-proxy
sudo cp /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-proxy-tool/target/release/proxy-v3-proxy-tool /ppaass-v3/build/ppaass-v3-tool
sudo cp /ppaass-v3/sourcecode/ppaass-v3/ppaass-v3-script/* /ppaass-v3/build/

sudo chmod 777 /ppaass-v3/build
cd /ppaass-v3/build
ls -l

sudo chmod 777 ppaass-v3-proxy
sudo chmod 777 ppaass-v3-tool
sudo chmod 777 *.sh
sudo dos2unix ./start-proxy.sh
sudo dos2unix ./concrete-start-proxy.sh
ulimit -n 65536
#Start with the low configuration by default
sudo ./start-proxy.sh



