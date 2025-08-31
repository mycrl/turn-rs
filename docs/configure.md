# Configure

Sample configuration file. However, please note that the sample is only used to show all configuration items. You need to adjust the corresponding configuration according to the actual situation. The configuration file is written in TOML format.

## Configuration keys

---

### `turn.realm`

-   Type: string
-   Default: "localhost"

This option describes the realm of the turn service. For the definition of realm, please refer to [RFC](https://datatracker.ietf.org/doc/html/rfc5766#section-3).

---

### `[turn.interfaces]`

-   Type: array of interface
-   Default: []

This option describes the interface to which the turn service is bound. A turn service can be bound to multiple interfaces at the same time.

---

### `[turn.interfaces.transport]`

-   Type: enum of string

Describes the transport protocol used by the interface. The value can be `udp` or `tcp`, which correspond to udp turn and tcp turn respectively, and choose whether to bind the turn service to a udp socket or a tcp socket.

---

### `[turn.interfaces.bind]`

-   Type: string

The IP address and port number bound to the interface. This is the address to which the internal socket is bound.

---

### `[turn.interfaces.external]`

-   Type: string

bind is used to bind to the address of your local NIC, for example, you have two NICs A and B on your server, the IP address of NIC A is 192.168.1.2, and the address of NIC B is 192.168.1.3, if you bind to NIC A, you should bind to the address of 192.168.1.2, and bind to 0.0.0.0 means that it listens to all of them at the same time.

external is that your network card for the client can "see" the ip address, continue the above example, your A network card in communication with the external, if it is in the local area network, then other clients see is your LAN address, that is, 192.168.1.2, but in reality, generally However, in reality, the network topology where the server is deployed, there will be another public ip, such as 1.1.1.1, which is your ip address seen by other clients.

As for why bind and external are needed, this is because for the stun protocol, the situation is more complicated, the stun server needs to inform its own external ip address, which allows the stun client to connect to the specified address through the ip address informed by the server.

---

### `api.bind`

-   Type: string
-   Default: "127.0.0.1:3000"

Describes the address to which the turn api server is bound.

The turn service provides an external REST API. External parties can control the turn service through HTTP or allow the turn service to perform dynamic authentication and push events to the outside through HTTP.

> Warning: The REST API does not provide any authentication or encryption measures. You need to run the turn service in a trusted network environment or add a proxy to increase authentication and encryption measures.

---

### `log.level`

-   Type: enum of string
-   Default: "info"

Describes the log level of the turn service. Possible values ​​are `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`.

---

### `auth.static_credentials`

-   Type: key values

Describes static authentication information, with username and password as key pair. Static identity authentication is authentication information provided to the turn service in advance. The turn service will first look for this table when it needs to authenticate the turn session. If it cannot find it, it will use Web Hooks for external authentication.

---

### `auth.static_auth_secret`

-   Type: string
-   Default: None

Static authentication key value (string) that applies only to the TURN REST API.

If set, the turn server will not request external services via the HTTP Hooks API to obtain the key.
