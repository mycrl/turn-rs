
### `/password?addr=[SocketAddr]&name=[String]` `GET`
> request external authentication.
> This interface will first try to find the internal static certificate table, if not found, then request the external interface for authentication.

Response: `String`  
Code: `200`

***

### `/events?kind=[kind]` `PUT`
> push event.
> Only subscribed events are pushed, other events are ignored.

Request:

### Allocated
> allocate request

| field | type       | description                             |
|-------|------------|-----------------------------------------|
| addr  | SocketAddr | node ip address                         |
| name  | String     | The name used for client authentication |
| port  | u16        | Request assigned port number            |

### Binding
> binding request

| field | type       | description                             |
|-------|------------|-----------------------------------------|
| addr  | SocketAddr | node ip address                         |

### ChannelBind
> channel binding request

| field  | type       | description                             |
|--------|------------|-----------------------------------------|
| addr   | SocketAddr | node ip address                         |
| name   | String     | The name used for client authentication |
| number | u16        | Request assigned channel number         |

### CreatePermission
> create permission request

| field | type       | description                             |
|-------|------------|-----------------------------------------|
| addr  | SocketAddr | node ip address                         |
| name  | String     | The name used for client authentication |
| relay | SocketAddr | Relay socket addr                       |

### Refresh
> refresh request

| field | type       | description                             |
|-------|------------|-----------------------------------------|
| addr  | SocketAddr | node ip address                         |
| name  | String     | The name used for client authentication |
| time  | u16        | Refresh time number                     |

### Abort
> node exit

| field  | type       | description                             |
|--------|------------|-----------------------------------------|
| addr   | SocketAddr | node ip address                         |
| name   | String     | The name used for client authentication |
