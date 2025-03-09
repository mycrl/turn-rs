# REST API

---

### GET - `/info` - Info

Info:

-   `software` - <sup>string</sup> - Software information of turn server
-   `uptime` - <sup>uint64</sup> - Turn the server's running time in seconds
-   `port_allocated` - <sup>uint16</sup> - The number of allocated ports
-   `port_capacity` - <sup>uint16</sup> - The total number of ports available for allocation
-   `interfaces` - <sup>Interface[]</sup> - Turn all interfaces bound to the server

Interface:

-   `transport` - <sup>int</sup> - 0 = UDP, 1 = TCP
-   `bind` - <sup>string</sup> - turn server listen address
-   `external` - <sup>string</sup> - specify the node external address and port

Get the information of the turn server, including version information, listening interface, startup time, etc.

---

### GET `/session?address=&interface=` - Session[]

Session:

-   `address` - <sup>string</sup> - The IP address and port number currently used by the session
-   `username` - <sup>string</sup> - Username used in session authentication
-   `channels` - <sup>uint16[]</sup> - Channel numbers that have been assigned to the session
-   `port` - <sup>uint16</sup> - Port numbers that have been assigned to the session
-   `expires` - <sup>uint32</sup> - The validity period of the current session application, in seconds
-   `permissions` - <sup>uint16[]</sup> - What ports have forwarding privileges for the session.

Get session information. A session corresponds to each UDP socket. It should be noted that a user can have multiple sessions at the same time.

---

### GET - `/session/statistics?address=&interface=` - Statistics

Statistics:

-   `received_bytes` - <sup>uint64</sup> - Number of bytes received in the current session
-   `send_bytes` - <sup>uint64</sup> - The number of bytes sent by the current session
-   `received_pkts` - <sup>uint64</sup> - Number of packets received in the current session
-   `send_pkts` - <sup>uint64</sup> - The number of packets sent by the current session

Get session statistics, which is mainly the traffic statistics of the current session.

---

### DELETE - `/session?address=&interface=`

Delete the session. Deleting the session will cause the turn server to delete all routing information of the current session. If there is a peer, the peer will also be disconnected.

---

### GET - `/events` (EventSource)

> This is an sse push event interface through which clients can subscribe to server events.

[Session]:

-   `address` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
-   `interface` - <sup>string</sup> - The network interface used by the current session.

allocate:

-   `session` - <sup>Session</sup>
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `port` - <sup>uint16</sup> - The port to which the request is assigned.

channel binding:

-   `session` - <sup>Session</sup>
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `channel` - <sup>uint16</sup> - The channel to which the request is binding.

create permission:

-   `session` - <sup>Session</sup>
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `ports` - <sup>uint16[]</sup> - The port number of the other side specified when the privilege was created.

refresh:

-   `session` - <sup>Session</sup>
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `lifetime` - <sup>uint32</sup> - Time to expiration in seconds.

closed:

-   `session` - <sup>Session</sup>
-   `username` - <sup>string</sup> - The username used for the turn session.
