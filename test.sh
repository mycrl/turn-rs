./turnutils_stunclient 192.168.31.62
./turnutils_peer -L 127.0.0.1 -L ::1 -L 0.0.0.0
./turnutils_uclient -u user1 -w test -n 100 -D -m 10 -l 171 -e 127.0.0.1 192.168.31.62
