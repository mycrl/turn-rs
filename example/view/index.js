const TURN_OPTIONS = {
    urls: "turn:localhost",
    credentialType: "password",
    credential: null,
    username: null,
}

const OPTIONS = {
    iceTransportPolicy: "relay",
    iceServers: [TURN_OPTIONS]
}

window.onload = () => new Vue({
    el: "#App",
    data: {
        peers: {},
        room: null,
        users: [],
        login: true,
        username: null,
        password: null,
        localStream: null,
        socket: new WebSocket("ws://localhost"),
        uid: String(Date.now())
    },
    watch: {
        async users(handle) {
            await this.delay(500)
            this.layout(handle.length)
        }
    },
    methods: {
        async start() {
            OPTIONS.iceServers[0].username = this.username
            OPTIONS.iceServers[0].credential = this.password
            this.localStream = await navigator.mediaDevices.getDisplayMedia()
            document.getElementById("self").srcObject = this.localStream
            this.login = false
            this.emit({
                type: "connected",
                broadcast: true,
            })
        },
        async onmessage({ data }) {
            let packet = JSON.parse(data)
            console.log(packet)
            if (packet.type === "users") {
                for (let u of packet.users) {
                    this.createOffer(u)   
                }
            } else
            if (packet.type === "icecandidate") {
                this.onIcecandidate(packet)
            } else
            if (packet.type === "answer") {
                this.onAnswer(packet)
            } else
            if (packet.type === "offer") {
                this.onOffer(packet)
            }
        },
        emit(payload) {
            this.socket.send(JSON.stringify({
                from: this.uid,
                ...payload
            }))
        },
        async onIcecandidate({ from, candidate }) {
            this.peers[from].addIceCandidate(candidate)
        },
        async onAnswer({ from, answer }) {
            const remoteDesc = new RTCSessionDescription(answer)
            this.peers[from].setRemoteDescription(remoteDesc)
        },
        async onOffer({ from, offer }) {
            this.createPeer(from)
            const remoteDesc = new RTCSessionDescription(offer)
            this.peers[from].setRemoteDescription(remoteDesc)
            const answer = await this.peers[from].createAnswer()
            await this.peers[from].setLocalDescription(answer)
            this.emit({ type: "answer", to: from, answer })
        },
        async createOffer(from) {
            this.createPeer(from)
            const offer = await this.peers[from].createOffer()
            await this.peers[from].setLocalDescription(offer)
            this.emit({ type: "offer", to: from, offer })
        },
        createPeer(name) {
            const remoteStream = new MediaStream()
            this.peers[name] = new RTCPeerConnection(OPTIONS)
            this.peers[name].addEventListener("track", ({ track }) => {
                remoteStream.addTrack(track, remoteStream)
            })
             
            this.peers[name].addEventListener("icecandidate", ({ candidate }) => {
                candidate && this.emit({ type: "icecandidate", to: name, candidate })
            })
            
            this.localStream.getTracks().forEach(track => {
                this.peers[name].addTrack(track, this.localStream)
            })
            
            this.users.push(name)
            this.peers[name].addEventListener("connectionstatechange", async event => {
                if (this.peers[name].connectionState === "connected") {
                    document.querySelector(`.Room #node-${name}`).srcObject = remoteStream
                }
            })
        },
        delay(timeout) {
            return new Promise(resolve => {
                setTimeout(resolve, timeout)
            })
        },
        layout(s) {
            const {clientWidth, clientHeight} = document.documentElement
            s == 0 && Object.assign(document.getElementById("self").style, {
                width: clientWidth + "px",
                height: clientHeight + "px"
            })
              
            if (s == 0) {
                return
            }
            
            s == 1 && this.style({ 
                width: clientWidth * 0.5 + "px",
                height: clientHeight + "px"
            })

            s == 1 && Object.assign(document.getElementById("self").style, {
                width: clientWidth * 0.5 + "px",
                height: clientHeight + "px"
            })
            
            if (s == 1) {
                return
            }
            
            const is_overflow = (s + 1) % 2 == 1
            const size = is_overflow ? s : s + 1
            const units = Math.sqrt(size)
            const is_float = units - Math.floor(units) > 0
            const w_units = is_float ? Math.floor(units) : units
                 
            let h_units = w_units
            for(;;h_units ++) {
                if (h_units * w_units === size) break
                if ((h_units + 1) * w_units > size) break
            }
            
            const width = clientWidth * (is_overflow ? 0.5 : 1) / w_units + "px"
            const height = clientHeight / h_units + "px"
            
            this.style({ width, height })
            Object.assign(document.getElementById("self").style, {
                width: is_overflow ? clientWidth * 0.5 + "px" : width,
                height: is_overflow ? clientHeight + "px" : height
            })
        },
        style(style) {
            this.users.map(i => `.Room #node-${i}`).forEach(path => {
                Object.assign(document.querySelector(path).style, style)  
            })
        }
    },
    created() {
        const {clientWidth, clientHeight} = document.documentElement
        document.getElementById("self").style.width = clientWidth + "px"
        document.getElementById("self").style.height = clientHeight + "px"
        this.socket.onmessage = this.onmessage.bind(this)
    }
})
