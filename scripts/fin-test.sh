# A script to test quickly

killall {node} &> /dev/null
rm -rf /tmp/*.db &> /dev/null
vals=(27000 27100 27200 27300)
tri=1000000

rand=$(shuf -i 1000-150000000 -n 1)
TESTDIR=${TESTDIR:="testdata/hyb_4"}
TYPE=${TYPE:="release"}
EXP=${EXP:-"appxcox_new"}
W=${W:="10000"}
curr_date=$(date +"%s%3N")
sleep=100
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
    --syncer $1 \
    --batch 10 \
    --rand $rand \
    --expo 2 > logs/syncer.log &

for((i=0;i<4;i++)); do
./target/$TYPE/node \
    --config $TESTDIR/nodes-$i.json \
    --ip ip_file \
    --sleep $st_time \
    --epsilon 10 \
    --delta 100 \
    --val ${vals[$i]} \
    --tri 100000 \
    --vsstype fin \
    --syncer $1 \
    --batch 10 \
    --rand $rand \
    --expo 2 > logs/$i.log &
done

# Kill all nodes sudo lsof -ti:7000-7015 | xargs kill -9