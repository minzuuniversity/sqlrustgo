#!/usr/bin/env bash
# #4956 A/B: full run. 3 rounds per scenario, A/B order alternated per
# round so monotonic machine drift cannot be mistaken for a side effect.
set -uo pipefail
cd /tmp/perf4956

run() {  # side round port scenario threads ops rows
  ./run_one.sh "/tmp/perf4956/server_$1" "$2" "$3" "$4" "$5" "$6" "$7" 2>/dev/null | grep '^RESULT|' \
    | sed "s/^/side=$1 round=$2 /"
}

for round in 1 2 3; do
  if [ $((round % 2)) -eq 1 ]; then ORDER="after before"; else ORDER="before after"; fi
  for side in $ORDER; do
    run "$side" "$round" $((13400 + round * 10)) S1 1 400 1
    run "$side" "$round" $((13401 + round * 10)) S2 8 200 1
    run "$side" "$round" $((13402 + round * 10)) S3 1 5 1000
  done
done
