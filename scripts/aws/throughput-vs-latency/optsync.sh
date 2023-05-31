ID=$1
TESTDIR=${2:-"testdata/b100-n3"}
DELAY=${3:-"50"}
CLI_TYPE=${4:-"client-apollo"}
SLEEP=${SLEEP:-"5"}

cd libchatter-rs

# sleep 30
# echo "Using arguments: --config $TESTDIR/nodes-$ID.json --ip ips_file --delta "$DELAY" -s"

./target/release/node-optsync \
    --config $TESTDIR/nodes-$ID.json \
    --sleep 140 \
    --ip ips_file &