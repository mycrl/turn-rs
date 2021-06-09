#!/bin/bash

COMMAND="turn --listen 0.0.0.0:3478"

if [ $TURN_REALM ]; then COMMAND="${COMMAND} --realm ${TURN_REALM}"; fi
if [ $TURN_NATS ]; then COMMAND="${COMMAND} --nats ${TURN_NATS}"; fi
if [ $TURN_BUFFER ]; then COMMAND="${COMMAND} --buffer ${TURN_BUFFER}"; fi
if [ $TURN_THREADS ]; then COMMAND="${COMMAND} --threads ${TURN_THREADS}"; fi
if [ $TURN_EXTERNAL ]; then COMMAND="${COMMAND} --external ${TURN_EXTERNAL}"; fi

/bin/bash -c "${COMMAND}"