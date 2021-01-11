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
        index: 0,
        rtc: null,
        streams: [],
        login: true,
        username: null,
        password: null,
        socket: new WebSocket("ws://localhost")
    },
    watch: {
        rtc(handle) {
            handle.addEventListener("track", this.track.bind(this))
            handle.addEventListener("icecandidate", this.icecandidate.bind(this))
            handle.addEventListener("connectionstatechange", this.connectionstatechange.bind(this))
        },
        async streams(handle) {
            await this.delay(500)
            this.layout(handle.length)
        }
    },
    methods: {
        async start() {
            OPTIONS.iceServers[0].username = this.username
            OPTIONS.iceServers[0].credential = this.password
            
            const localStream = await navigator.mediaDevices.getDisplayMedia()
            this.rtc = new RTCPeerConnection(OPTIONS)
            this.socket.onmessage = this.message.bind(this)
            this.login = false
            
            document.getElementById("self").srcObject = localStream
            localStream.getTracks().forEach(track => {
                this.rtc.addTrack(track, localStream)
            })

            this.send({
                username: this.username,
                type: "connection"
            })
        },
        track(event) {
            const remoteStream = new MediaStream()
            remoteStream.addTrack(event.track, remoteStream)
            this.append(remoteStream)
        },
        icecandidate(event) {
            event.candidate && this.send({
                iceCandidate: event.candidate,
                type: "iceCandidate"
            })
        },
        connectionstatechange(event) {
            if (this.rtc.connectionState === "connected") {
                
            }
        },
        message({ data }) {
            const payload = JSON.parse(data)
            this["on" + payload.type](payload)
        },
        send(payload) {
            this.socket.send(JSON.stringify(payload))
        },
        async onconnection({ username }) {
            const offer = await this.rtc.createOffer()
            await this.rtc.setLocalDescription(offer)
            this.send({ 
                type: "offer", 
                offer 
            })
        },
        async onoffer({ offer }) {
            this.rtc.setRemoteDescription(new RTCSessionDescription(offer))
            const answer = await this.rtc.createAnswer()
            await this.rtc.setLocalDescription(answer)
            this.send({
                type: "answer",
                answer
            })
        },
        async onanswer({ answer }) {
            const remoteDesc = new RTCSessionDescription(answer)
            await this.rtc.setRemoteDescription(remoteDesc)
        },
        async oniceCandidate({ iceCandidate }) {
            await this.rtc.addIceCandidate(iceCandidate)
        },
        async append(stream) {
            this.index += 1
            this.streams.push(this.index)
            await this.delay(500)
            const id = `.Room #node-${this.index}`
            document.querySelector(id).srcObject = stream
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
            this.streams.map(i => `.Room #node-${i}`).forEach(path => {
                Object.assign(document.querySelector(path).style, style)  
            })
        }
    },
    created() {
        const {clientWidth, clientHeight} = document.documentElement
        document.getElementById("self").style.width = clientWidth + "px"
        document.getElementById("self").style.height = clientHeight + "px"
    }
})
