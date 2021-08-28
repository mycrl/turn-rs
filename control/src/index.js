"use strict"

/** 
 * @module Control 
 */

const Nats = require("nats")
const { EventEmitter } = require("events")
const Subscription = require("./subscription")

/**
 * @extends EventEmitter
 * @class
 */
module.exports = class Control extends EventEmitter {

    /**
     * @param {strings} options.server - nats server url.
     * @constructor
     */
    constructor(options) {
        super()
        this._inner = null
        this._proxy = null
        this._options = options
        this._init()
    }

    /**
     * init Control.
     * @returns {Promise<void>}
     * @private
     */
    async _init() {
        const { servers } = this._options
        this._inner = await Nats.connect({ servers })
        this.emit("ready", undefined)
    }

    /**
     * @public
     * @example
     * new Control({
     *     server: "localhost:4222"
     * }).Broker.auth
     */
    get Broker() {
        if (this._proxy) return this._proxy
        return this._proxy = new Proxy({}, {
            get: (_, key) => new Subscription(
                this._inner.subscribe(key + ".>")
            )
        })
    }
}
