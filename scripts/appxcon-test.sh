# A script to test quickly

killall {node} &> /dev/null
rm -rf /tmp/*.db &> /dev/null
vals=(27000 27100 27200 27300)
tri=1000000

TESTDIR=${TESTDIR:="testdata/hyb_4"}
TYPE=${TYPE:="release"}
EXP=${EXP:-"appxcox_new"}
W=${W:="10000"}
curr_date=$(date +"%s%3N")
sleep=$1
st_time=$((curr_date+sleep))
echo $st_time
# Run the syncer now
./target/$TYPE/node \
    --config $TESTDIR/nodes-0.json \
    --ip ip_file \
    --sleep $st_time \
    --vsstype sync \
    --epsilon 10 \
    --delta 5000 \
    --val 100 \
    --tri $tri \
    --syncer $4 \
    --batch $5 > logs/syncer.log &

for((i=0;i<4;i++)); do
./target/$TYPE/node \
    --config $TESTDIR/nodes-$i.json \
    --ip ip_file \
    --sleep $st_time \
    --epsilon $1 \
    --delta $2 \
    --val ${vals[$i]} \
    --tri $tri \
    --vsstype $3 \
    --syncer $4 \
    --batch $5 > logs/$i.log &
done

# Client has finished; Kill the nodes
killall ./target/$TYPE/appxcox_new &> /dev/null

# Kill all nodes sudo lsof -ti:7000-7015 | xargs kill -9
