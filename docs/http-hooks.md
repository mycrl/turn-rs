# Web Hooks

#### Global Request Headers

* `realm` - <sup>string</sup> - turn server realm
* `rid` - <sup>string</sup> - The runtime ID of the turn server

rid: A new ID is generated each time the server is started. This is a random string. Its main function is to determine whether the turn server has been restarted.

***

### GET - `/password?addr=&name=`

Get the current user's password, which is mainly used to provide authentication for the turn server.

***

### POST - `/events` - Events

binding request:

* `kind` - <sup>string</sup> - "binding"
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.

allocate request:

* `kind` - <sup>string</sup> - "allocated"
* `name` - <sup>string</sup> - The username used for the turn session.
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
* `port` - <sup>uint16</sup> - The port to which the request is assigned.

channel binding request:

* `kind` - <sup>string</sup> - "channel_bind"
* `name` - <sup>string</sup> - The username used for the turn session.
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
* `channel` - <sup>uint16</sup> - The channel to which the request is binding.

create permission request:

* `kind` - <sup>string</sup> - "create_permission"
* `name` - <sup>string</sup> - The username used for the turn session.
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
* `relay` - <sup>uint16</sup> - The port number of the other side specified when the privilege was created.

refresh request:

* `kind` - <sup>string</sup> - "refresh"
* `name` - <sup>string</sup> - The username used for the turn session.
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
* `expiration` - <sup>uint32</sup> - Time to expiration in seconds.

session closed:

* `kind` - <sup>string</sup> - "abort"
* `name` - <sup>string</sup> - The username used for the turn session.
* `addr` - <sup>string</sup> - The IP address and port number of the UDP or TCP connection used by the client.
