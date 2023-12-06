#!/bin/bash
for i in {1..3}
do
	echo "Running iteration $i"
	fab rerun
	sleep 40s
	fab logs
	mv syncer.log syncer-$i.log
	fab kill
done
