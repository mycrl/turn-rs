const { createApp, ref, watch } = Vue;

const app = createApp({
    setup() {
        const localControls = ref({});
        const remoteControls = ref({});
        const running = ref(false);

        return {
            localControls,
            remoteControls,
            running,
            async start() {
                if (running.value) return;

                const localVideo = document.querySelector("#local video, .panel:first-child video");
                const remoteVideo = document.querySelector(".panel:last-child video");

                console.log({
                    urls: `turn:${localControls.value.server}:3478?transport=${localControls.value.transport}`,
                    username: localControls.value.username,
                    credential: "test",
                });

                /**
                 * This is a local RTC connection that forces the use of forwarding.
                 * you can configure the ice server parameters yourself.
                 */
                let localRtc = new RTCPeerConnection({
                    iceTransportPolicy: "relay",
                    iceServers: [
                        {
                            urls: `turn:${localControls.value.server}:3478?transport=${localControls.value.transport}`,
                            username: localControls.value.username,
                            credential: "test",
                        },
                    ],
                });
                /**
                 * This is a remote RTC connection that forces the use of forwarding.
                 * you can configure the parameters of the ice server yourself.
                 */
                let remoteRtc = new RTCPeerConnection({
                    iceTransportPolicy: "relay",
                    iceServers: [
                        {
                            urls: `turn:${remoteControls.value.server}:3478?transport=${remoteControls.value.transport}`,
                            username: remoteControls.value.username,
                            credential: "test",
                        },
                    ],
                });
                /**
                 * The local ice candidate is added to the remote session.
                 */
                localRtc.addEventListener("icecandidate", async (event) => {
                    await remoteRtc.addIceCandidate(event.candidate);
                });
                /**
                 * The remote ice candidate is added to the local session.
                 */
                remoteRtc.addEventListener("icecandidate", async (event) => {
                    await localRtc.addIceCandidate(event.candidate);
                });
                /**
                 * Create a media stream and add all audio and video tracks received
                 * by the remote session to this media stream.
                 */
                {
                    const remoteStream = new MediaStream();
                    remoteVideo.srcObject = remoteStream;
                    remoteRtc.addEventListener("track", (event) => {
                        remoteStream.addTrack(event.track);
                    });
                }
                /**
                 * Get the capture stream of the desktop or tab, here it is
                 * capturing video not audio.
                 */
                const stream = await navigator.mediaDevices.getDisplayMedia({
                    video: true,
                });
                running.value = true;
                stream.getVideoTracks()[0].addEventListener("ended", () => {
                    running.value = false;
                });
                /**
                 * The local video player previews the captured stream.
                 */
                localVideo.srcObject = stream;
                /**
                 * The local capture streams are all added to the local RTC
                 * session.
                 */
                for (const track of stream.getTracks()) {
                    localRtc.addTrack(track, stream);
                }
                /**
                 * The local connection creates the OFFER to submit to the
                 * remote connection.
                 */
                const offer = await localRtc.createOffer();
                await localRtc.setLocalDescription(offer);
                await remoteRtc.setRemoteDescription(offer);
                /**
                 * Remote connections create ANSWER submissions to the local
                 * connection.
                 */
                const answer = await remoteRtc.createAnswer();
                await remoteRtc.setLocalDescription(answer);
                await localRtc.setRemoteDescription(answer);
            },
        };
    },
});

app.component("Controls", {
    template: document.getElementById("Controls").innerHTML,
    emits: ["change"],
    setup(props, context) {
        const server = ref("127.0.0.1");
        const transport = ref("udp");
        const username = ref("user1");

        context.emit("change", { server, transport, username });

        watch([server, transport, username], () => {
            context.emit("change", { server, transport, username });
        });

        return {
            server,
            transport,
            username,
        };
    },
});

app.mount("#app");
