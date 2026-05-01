#!/bin/bash
./build/typd 2> typd_debug.log &
TYPD_PID=$!
sleep 2
# Click 'A' (approx)
# Surface local: (96, 177)
# Screen local (assuming 1080p): (96, 879)
ydotool mousemove -a 96 879
ydotool click 0xC0
sleep 0.1
ydotool click 0x40
sleep 1
kill $TYPD_PID
wait $TYPD_PID 2>/dev/null
