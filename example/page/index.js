new Vue({
    el: '#App',
    data: {
        peers: {},
        room: null,
        login: true,
        username: String(Date.now()),
        password: null,
        localStream: null,
        socket: null,
        uid: String(Date.now()),
        domain: location.host,
        style: {},
        isOffer: false
    },
    methods: {
        async start() {
            this.localStream = await navigator.mediaDevices.getUserMedia({ 
                video: true,
                audio: true
            })

            this.socket = new WebSocket('ws://' + this.domain)
            this.socket.onmessage = this.onmessage.bind(this)
            this.socket.onopen = () => {
                this.emit({ type: 'connected', broadcast: true })
                this.login = false
            }
        },
        async onmessage({ data }) {
            let packet = JSON.parse(data)
            console.log(packet)
            if (packet.type === 'users') {
                for (let u of packet.users) {
                    this.createOffer(u)
                }
            } else
            if (packet.type === 'icecandidate') {
                this.onIcecandidate(packet)
            } else
            if (packet.type === 'answer') {
                this.onAnswer(packet)
            } else
            if (packet.type === 'offer') {
                this.onOffer(packet)
            }
        },
        emit(payload) {
            console.log(payload)
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
            this.emit({ type: 'answer', to: from, answer })
        },
        async createOffer(from) {
            this.createPeer(from)
            const offer = await this.peers[from].createOffer()
            await this.peers[from].setLocalDescription(offer)
            this.emit({ type: 'offer', to: from, offer })
        },
        createPeer(name) {
            const remoteStream = new MediaStream()
            this.peers[name] = new RTCPeerConnection({
                // iceTransportPolicy: 'relay',
                iceServers: [{
                    urls: 'turn:render.psyai.net',
                    credentialType: 'password',
                    credential: this.password,
                    username: this.username,
                }]
            })
            
            this.peers[name].addEventListener('track', ({ track }) => {
                remoteStream.addTrack(track, remoteStream)
            })

            this.peers[name].addEventListener('icecandidate', ({ candidate }) => {
                candidate && this.emit({ type: 'icecandidate', to: name, candidate })
            })

            if (this.localStream) {
                this.localStream.getTracks().forEach(track => {
                    if (track.kind === "audio") {
                        this.peers[name].addTrack(track, this.localStream)   
                    }
                })
            }

            this.peers[name].addEventListener('connectionstatechange', async event => {
                if (this.peers[name].connectionState === 'connected') {
                    document.getElementById('remote').srcObject = remoteStream
                }
            })
        }
    }
})
