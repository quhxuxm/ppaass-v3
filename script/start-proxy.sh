#!/bin/sh
a=0
while [ $a -lt 5 ];
do
    process_id=$(ps -ef | grep "ppaass-v3-proxy" | grep -v grep | awk '{print $2}')
    if [ -z "$process_id"]; then
        echo "No ppaass-v3-proxy process"
    else
        echo "Found ppaass-v3-proxy process: $process_id"
        kill -9 $process_id
        break
    fi
    a=`expr $a + 1`
    sleep 2
done
ulimit -n 65536
sudo nohup ./concrete-start-proxy.sh >run.log 2>&1 &