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
* `addr` - <sup>string</sup>

allocate request:

* `kind` - <sup>string</sup> - "allocated"
* `name` - <sup>string</sup>
* `addr` - <sup>string</sup>
* `port` - <sup>uint16</sup>

channel binding request:

* `kind` - <sup>string</sup> - "channel_bind"
* `name` - <sup>string</sup>
* `addr` - <sup>string</sup>
* `channel` - <sup>uint16</sup>

create permission request:

* `kind` - <sup>string</sup> - "create_permission"
* `name` - <sup>string</sup>
* `addr` - <sup>string</sup>
* `channel` - <sup>uint16</sup>

refresh request:

* `kind` - <sup>string</sup> - "refresh"
* `name` - <sup>string</sup>
* `addr` - <sup>string</sup>
* `expiration` - <sup>uint32</sup>

session closed:

* `kind` - <sup>string</sup> - "abort"
* `name` - <sup>string</sup>
* `addr` - <sup>string</sup>
