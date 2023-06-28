import { useState, useEffect } from 'react'
import './App.css'

function Interface({ stats }) {
    return (
        <>
            <h5>TURN Server Interfaces:</h5>
            <table>
                <thead>
                    <tr>
                        <th>transport</th>
                        <th>interface</th>
                        <th>external ip</th>
                    </tr>
                </thead>
                <tbody>
                    { stats.interfaces.map((item, index) => {
                         return <tr key={index}>
                            <td>{ item.transport }</td>
                            <td>{ item.bind }</td>
                            <td>{ item.external }</td>
                        </tr>     
                    }) }
                </tbody>
            </table>
        </>
    )
}

function NodePupBox({ 
    addr, 
    onClose = () => {} 
}) {
    const [node, setNode] = useState({
        allocated_channels: [],
        allocated_ports: [],
    })
    
    useEffect(() => {
        if (addr) {
            fetch('http://localhost:3000/node?addr=' + addr)
                .then(async res => {
                    setNode(await res.json())
                })
        }
    }, [])
    
    const removeNode = async () => {
        await fetch('http://localhost:3000/node?addr=' + addr, {
            method: 'DELETE'
        })
        
        onClose()
    }
    
    return (
        <div id="nodePopBox">
            <div>
                <h5>Node Info:</h5>
                <hr/>
                
                <table>
                    <thead>
                        <tr>
                            <th>addr</th>
                            <th>username</th>
                            <th>password</th>
                            <th>lifetime</th>
                            <th>timer</th>
                            <th>allocated channels</th>
                            <th>allocated ports</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>{ addr }</td>
                            <td>{ node.username }</td>
                            <td>{ node.password }</td>
                            <td>{ node.lifetime }</td>
                            <td>{ node.timer }</td>
                            <td>{ node.allocated_channels.join(' | ') }</td>
                            <td>{ node.allocated_ports.join(' | ') }</td>
                        </tr>
                    </tbody>
                </table>
                <hr/>
                
                <div id="ctrl">
                    <button 
                        style={{marginRight: '10px'}}
                        onClick={onClose}
                    >cancel</button>
                    <button onClick={removeNode}>disconnect</button>
                </div>
            </div>
        </div>
    )
}

function Sockets() {
    const [report, setReport] = useState({})
    const [range, setRange] = useState({})
    const [addr, setAddr] = useState(null)
    
    useEffect(() => {
        let old = {}
        const loop = setInterval(async () => {
            const ret = await fetch('http://localhost:3000/report')
                .then(res => res.json())
            const report = ret.reduce((obj, [k, v]) => {
                return Object.assign(obj, { [k]: v })
            }, {})
            
            setReport(report)
            setRange(Object.keys(report).map(key => {
                return [
                    key,
                    {
                        received_bytes_avg: report[key].received_bytes - (old[key] || {}).received_bytes || 0,
                        send_bytes_avg: report[key].send_bytes - (old[key] || {}).send_bytes || 0,
                        received_pkts_avg: report[key].received_pkts - (old[key] || {}).received_pkts || 0,
                        send_pkts_avg: report[key].send_pkts - (old[key] || {}).send_pkts || 0,
                    }
                ]
            }).reduce((obj, [k, v]) => Object.assign(obj, { [k]: v }), {}))
            old = report
        }, 1000)
        
        return () => {
            clearInterval(loop)
        }
    }, [])
    
    return (
        <>
            { addr ? <NodePupBox 
                addr={addr} 
                onClose={() => setAddr(null)}
            /> : null }
             <table>
                <thead>
                    <tr>
                        <th>socket addr</th>
                        <th>received kbytes</th>
                        <th>received bytes/s</th>
                        <th>send kbytes</th>
                        <th>send bytes/s</th>
                        <th>received packages</th>
                        <th>received packages/s</th>
                        <th>send packages</th>
                        <th>send packages/s</th>
                    </tr>
                </thead>
                <tbody>
                    { Object.keys(report).map(key => {
                        return <tr key={key}>
                            <td>
                                <a href="#" 
                                    key={key} 
                                    onClick={() => setAddr(key)}
                                >{ key }</a>
                            </td>
                            <td>{ report[key].received_bytes }</td>
                            <td>{ range[key].received_bytes_avg * 1024 }</td>
                            <td>{ report[key].send_bytes }</td>
                            <td>{ range[key].send_bytes_avg * 1024 }</td>
                            <td>{ report[key].received_pkts }</td>
                            <td>{ range[key].received_pkts_avg }</td>
                            <td>{ report[key].send_pkts }</td>
                            <td>{ range[key].send_pkts_avg }</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </>
    )
}

function Users() {
    const [users, setUsers] = useState([])
    const [addr, setAddr] = useState(null)
    
    useEffect(() => {
        fetch('http://localhost:3000/users')
            .then(async res => {
                setUsers(await res.json())
            })
    }, [])
    
    return (
        <>
            { addr ? <NodePupBox 
                addr={addr} 
                onClose={() => setAddr(null)}
            /> : null }
            <table>
                <thead>
                    <tr>
                        <th>username</th>
                        <th>socket addrs</th>
                    </tr>
                </thead>
                <tbody>
                    { users.map(([user, addrs]) => {
                        return <tr key={user}>
                            <td>{ user }</td>
                            <td>{ addrs.map(addr => {
                                return <a href="#" 
                                    key={addr} 
                                    onClick={() => setAddr(addr)}
                                >{ addr }</a>       
                            }) }</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </>
    )
}

function Tabs() {
    const [index, setIndex] = useState(0)
    
    const actionId = (value) => {
        return index == value ? 'tabAction' : null
    }
    
    return (
        <>
            <div className="tabs">
            { ['Sockets', 'Users'].map((k, i) => {
                return <button 
                    key={i}
                    id={actionId(i)} 
                    onClick={() => setIndex(i)}
                >{ k }</button>
            }) }
            </div>
            <hr style={{marginTop: 0}}/>
            { index == 0 ? <Sockets/> : <Users/> }
        </>
    )
}

function ServerInfo({ stats }) {
    return (
        <>
            <h5>TURN Server Info:</h5>
            <table>
                <thead>
                    <tr>
                        <th>software</th>
                        <th>realm</th>
                        <th>uptime</th>
                        <th>ports capacity</th>
                        <th>allocated ports</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{ stats.software }</td>
                        <td>{ stats.realm }</td>
                        <td>{ stats.uptime }s</td>
                        <td>{ stats.port_capacity }</td>
                        <td>{ stats.port_allocated }</td>
                    </tr>
                </tbody>
            </table>
        </>
    )
}

export default function() {
    const [stats, setStats] = useState({
        interfaces: []
    })
    
    useEffect(() => {
        fetch('http://localhost:3000/stats')
            .then(async res => {
                setStats(await res.json())
            })
    }, [])

    return (
        <figure>
            <div>
                <ServerInfo stats={stats}/>
                <hr/>
                <Interface stats={stats}/>
                <hr/>
                <Tabs/>
                <hr/>
            </div>
        </figure>
    )
}