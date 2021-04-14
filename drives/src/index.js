"use strict"

import Nats from "nats"
import { EventEmitter } from "events"
import Subscription from "./subscription"

/** 
 * @module Mysticeti 
 */

/**
 * @extends EventEmitter
 * @class
 */
export default class Mysticeti extends EventEmitter {
    
    /**
     * @param {string} options.server - nats server url.
     * @constructor
     */
    constructor(options) {
        super()
        this._inner = null
        this._proxy = null
    }
    
    /**
     * 
     * @example
     * new Mysticeti({
     *     server: "localhost:4222"
     * }).Broker.auth
     */
    get Broker() {
        return this._proxy || 
            this._proxy = new Proxy({}, {
                get: (_, key) => new Subscription(
                    this._inner.subscribe(key + ".*")
                )
            })
    }
}
