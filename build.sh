#!/bin/bash


if [ $# -gt 0 ]
then
    if [ ! -d "$1/quasipaa" ]; 
    then
        cargo build --release
        mkdir "$1/quasipaa"
        cp ./target/release/quasipaa "$1/quasipaa"
        cp ./configure.toml "$1/quasipaa"
        echo "Build Ok:"
        echo "    project: $1/quasipaa"
    else
        echo "Error:"
        echo "    project directory already exists!"
    fi
else
    echo "Missing parameters:"
    echo "    undefined output directory!"
    echo "    help: build.sh ~/"
fi