# A script to test quickly

make artemis
make artemis-release
killall node-artemis &> /dev/null

TESTDIR=${TESTDIR:="testdata/b100-n3"}
TYPE=${TYPE:="release"}
W=${W:="80000"}

./target/$TYPE/node-artemis \
    --config $TESTDIR/nodes-0.json \
    --ip ip_file \
    --sleep 10 \
    -s $1 &> 0.log &
./target/$TYPE/node-artemis \
    --config $TESTDIR/nodes-1.json \
    --ip ip_file \
    --sleep 10 \
    -s $1 &> 1.log &
./target/$TYPE/node-artemis \
    --config $TESTDIR/nodes-2.json \
    --ip ip_file \
    --sleep 10 \
    -s $1 &> 2.log &

sleep 15
# Nodes must be ready by now
./target/$TYPE/client-artemis \
    --config $TESTDIR/client.json \
    -i cli_ip_file \
    -w $W \
    -m 1000000 $1

# Client has finished; Kill the nodes
killall ./target/$TYPE/node-artemis &> /dev/null