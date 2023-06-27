import { useState, useEffect } from 'react'
import './App.css'

async function getStats() {
    return await fetch('http://localhost:3000/stats')
        .then(res => res.json())
}

async function getReport() {
    return await fetch('http://localhost:3000/report')
        .then(res => res.json())
}

async function getUsers() {
    return await fetch('http://localhost:3000/users')
        .then(res => res.json())
}

function Interface(props) {
    return (
        <tr>
            <td>{ props.interface.transport }</td>
            <td>{ props.interface.bind }</td>
            <td>{ props.interface.external }</td>
        </tr>
    )
}

function Sockets() {
    const [report, setReport] = useState({})
    const [range, setRange] = useState({})
    
    useEffect(() => {
        let old = {}
        const loop = setInterval(async () => {
            let report = (await getReport())
                .reduce((obj, [k, v]) => Object.assign(obj, { [k]: v }), {})
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
                            <a href="#">{ key }</a>
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
    )
}

function Users() {
    const [users, setUsers] = useState({
        interfaces: []
    })
    
    useEffect(() => {
        getUsers().then(users => {
            setUsers(users)
        })
    }, [])
    
    return (
        <table>
            <thead>
                <tr>
                    <th>username</th>
                    <th>socket addrs</th>
                </tr>
            </thead>
            <tbody>
                { Object.keys(users).map(key => {
                    return <tr key={key}>
                        <td>{ key }</td>
                        <td>{ users[key].map(addr => {
                            return <a href="#">{ addr }</a>       
                        }) }</td>
                    </tr>
                }) }
            </tbody>
        </table>
    )
}

function Tabs() {
    const [index, setIndex] = useState(1)
    
    const actionId = (value) => {
        return index == value ? 'tabAction' : null
    }
    
    return (
        <>
            <div className="tabs">
                <button id={actionId(0)} onClick={() => setIndex(0)}>Sockets</button>
                <button id={actionId(1)} onClick={() => setIndex(1)}>Users</button>
            </div>
            <hr style={{marginTop: 0}}/>
            { index == 0 ? <Sockets/> : <Users/> }
        </>
    )
}

export default function() {
    const [stats, setStats] = useState({
        interfaces: []
    })
    
    useEffect(() => {
        getStats().then(stats => {
            setStats(stats)
        })
    }, [])

    return (
        <figure>
            <div>
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
                <hr/>

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
                             return <Interface key={index} interface={item}/>            
                        }) }
                    </tbody>
                </table>
                <hr/>

                <Tabs/>
                <hr/>
            </div>
        </figure>
    )
}