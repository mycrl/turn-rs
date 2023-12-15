'use strict'

const { Service, Observer } = require('./')

class TurnObserver extends Observer {
    async get_password_async(addr, name) {
        return 'test'
    }
}

const service = new Service('test', ['127.0.0.1:7890'], new TurnObserver())
const processer = service.get_processer('127.0.0.1:7890', '127.0.0.1:7890')

console.log('start process')
processer.process(Buffer.from('test'), '192.168.0.1:7890')
    .then(ret => {
        console.log('ret', ret)
    })
    .catch(e => {
        console.error('error', e)
    })