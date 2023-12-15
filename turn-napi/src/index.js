'use strict'

const { TurnService } = require('../build/Release/turn.node')

class Observer {
    get_password(addr, name, callback) {
        this.get_password_async(addr, name)
            .then(ret => callback(false, ret))
            .catch(_ => callback(true))
    }
}

class Processer {
    constructor(processer) {
        this._processer = processer
    }
    
    process(buf, addr) {
        return new Promise((resolve, reject) => {
            this._processer.process(buf, addr, (is_err, ret) => {
                is_err ? reject(ret) : resolve(ret)
            })
        })
    }
}

class Service {
    constructor(realm, externals, observer) {
        this._service = new TurnService(realm, externals, observer)
    }
    
    get_processer(inter, external) {
        return new Processer(this._service.get_processer(inter, external))
    }
}

module.exports = {
    Service,
    Observer,
    Processer,
}