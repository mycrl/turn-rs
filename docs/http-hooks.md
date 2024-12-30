# Web Hooks

#### Global Request Headers

-   `realm` - <sup>string</sup> - turn server realm
-   `nonce` - <sup>string</sup> - The runtime id of the turn server

nonce: A new ID is generated each time the server is started. This is a random string. Its main function is to determine whether the turn server has been restarted.

---

### GET - `/password?address=&interface=&transport=&username=`

Get the current user's password, which is mainly used to provide authentication for the turn server.

---

### POST - `/events` - Events

[Session]:

-   `address` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
-   `interface` - <sup>string</sup> - The network interface used by the current session.

---

allocate request:

-   `session` - <sup>Session</sup>
-   `kind` - <sup>string</sup> - "allocated"
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `port` - <sup>uint16</sup> - The port to which the request is assigned.

channel binding request:

-   `session` - <sup>Session</sup>
-   `kind` - <sup>string</sup> - "channel_bind"
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `channel` - <sup>uint16</sup> - The channel to which the request is binding.

create permission request:

-   `session` - <sup>Session</sup>
-   `kind` - <sup>string</sup> - "create_permission"
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `ports` - <sup>uint16[]</sup> - The port number of the other side specified when the privilege was created.

refresh request:

-   `session` - <sup>Session</sup>
-   `kind` - <sup>string</sup> - "refresh"
-   `username` - <sup>string</sup> - The username used for the turn session.
-   `lifetime` - <sup>uint32</sup> - Time to expiration in seconds.

session closed:

-   `session` - <sup>Session</sup>
-   `kind` - <sup>string</sup> - "abort"
-   `username` - <sup>string</sup> - The username used for the turn session.
