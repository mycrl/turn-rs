import mystical from "./src/index.js"

const mysticeti = new mystical({
    server: "localhost:4222"
})

mysticeti.on("ready", () => {
    mysticeti.Broker.auth.handler(message => {
        console.log(message)
        return {
            password: "panda",
            group: 0
        }
    })
})
