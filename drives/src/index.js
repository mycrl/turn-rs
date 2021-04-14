"use strict"

import Nats from "nats"

/**
 * 
 * @class
 */
export default class Mysticeti {
    
    /**
     * @param {string} [server] nats server url.
     * @constructor
     */
    constructor() {
        this.inner = null
        this.brokers = {}
        this.broker_handle = null
    }
    
    get Broker() {
        if (!this.broker_handle) {
            this.broker_handle = new Proxy({}, {
                get: (_, key) => {
                    
                }
            })
        }
    }
}
