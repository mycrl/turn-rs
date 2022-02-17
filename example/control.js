'use strict'

const Control = require('control')

const control = new Control({
    servers: 'localhost:4222'
})

control.on('ready', () => {
    control.Broker.auth.handler(message => {
        console.log(message)
        return {
            password: 'panda',
            group: 0
        }
    })
})
