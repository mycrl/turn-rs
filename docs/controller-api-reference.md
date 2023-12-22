### `/stats` `GET`
> get server status.

Response:

| field            | type       | description                                             |
|------------------|------------|---------------------------------------------------------|
| software         | String     | Software information, usually a name and version string |
| bind_address     | SocketAddr | The listening address of the turn server                |
| external_address | SocketAddr | The external address of the turn server                 |
| uptime           | u64        | The running time of the server, in seconds              |
| port_capacity    | u16        | Turn server port pool capacity                          |
| port_allocated   | u16        | The number of ports that the turn server has classified |
| realm            | String     | The partition where the turn server resides             |

***

### `/report` `GET`
> Get a list of workers.
> Workers are bound to the internal threads of the server. Through this interface, you can get how many threads currently exist in the server, and how much data processing capacity each thread has.

Response: `Store[]`

Store:

| field          | type | description                                                               |
|----------------|------|---------------------------------------------------------------------------|
| received_pkts  | u64  | The total number of data packets that the turn server has received so far |
| send_pkts      | u64  | The total number of packets sent by the turn server so far                |
| received_bytes | u64  | The total number of data bytes that the turn server has received so far   |
| send_bytes     | u64  | The total number of bytes sent by the turn server so far                  |

***

### `/users?skip=[number]&limit=[number]` `GET`
> get user list.
> This interface returns the username and a list of addresses used by this user.

Response: `{ [key: string]: SocketAddr[] }`

***

### `/node?addr=[SocketAddr]` `GET`
> Get node information.
> This interface can obtain the node basic information and assigned information, including the survival time.

Response:

| field              | type   | description                                     |
|--------------------|--------|-------------------------------------------------|
| username           | String | Username for the current session                |
| password           | String | The user key for the current session            |
| lifetime           | u64    | The lifetime of the current user                |
| timer              | u64    | The active time of the current user, in seconds |
| allocated_channels | u16[]  | List of assigned channel numbers                |
| allocated_ports    | u16[]  | List of assigned port numbers                   |

***

### `/node?addr=[SocketAddr]` `DELETE`
> Delete a node under the user.
> This will cause all information of the current node to be deleted, including the binding relationship, and at the same time terminate the session of the current node and stop forwarding data.

Response: `bool`