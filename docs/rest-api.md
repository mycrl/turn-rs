# REST API

#### Global Response Headers

* `realm` - <sup>string</sup> - turn server realm
* `rid` - <sup>string</sup> - The runtime ID of the turn server

rid: A new ID is generated each time the server is started. This is a random string. Its main function is to determine whether the turn server has been restarted.

***

### GET - `/info` - Info

Info:

* `software` - <sup>string</sup> - Software information of turn server
* `uptime` - <sup>uint64</sup> - Turn the server's running time in seconds
* `port_allocated` - <sup>uint16</sup> - The number of allocated ports
* `port_capacity` - <sup>uint16</sup> - The total number of ports available for allocation
* `interfaces` - <sup>Interface[]</sup> - Turn all interfaces bound to the server

Interface:

* `transport` - <sup>int</sup> - 0 = UDP, 1 = TCP
* `bind` - <sup>string</sup> - turn server listen address
* `external` - <sup>string</sup> - specify the node external address and port

Get the information of the turn server, including version information, listening interface, startup time, etc.

***

### GET `/session?addr=&username=` - Session[]

Session:

* `address` - <sup>string</sup> - The IP address and port number currently used by the session
* `username` - <sup>string</sup> - Username used in session authentication
* `password` - <sup>string</sup> - The password used in session authentication
* `channel?` - <sup>uint16</sup> - Channel numbers that have been assigned to the session
* `port?` - <sup>uint16</sup> - Port numbers that have been assigned to the session
* `expiration` - <sup>uint32</sup> - The validity period of the current session application, in seconds
* `lifetime` - <sup>uint32</sup> - The lifetime of the session currently in use, in seconds

Get session information. A session corresponds to each UDP socket. It should be noted that a user can have multiple sessions at the same time.

***

### GET - `/session/statistics?addr=` - Statistics

Statistics:

* `received_bytes` - <sup>uint64</sup> - Number of bytes received in the current session/s
* `send_bytes` - <sup>uint64</sup> - The number of bytes sent by the current session/s 
* `received_pkts` - <sup>uint64</sup> - Number of packets received in the current session/s
* `send_pkts` - <sup>uint64</sup> - The number of packets sent by the current session/s
* `error_pkts` - <sup>uint64</sup> - The number of packets error by the current session/s

Get session statistics, which is mainly the traffic statistics of the current session.

***

### DELETE - `/session?addr=&username=`

Delete the session. Deleting the session will cause the turn server to delete all routing information of the current session. If there is a peer, the peer will also be disconnected.
