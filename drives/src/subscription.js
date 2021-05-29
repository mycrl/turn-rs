"use strict"

import Nats from "nats"

/** 
 * @module mystical/Subscription
 */

/**
 * Nats Subscription.
 * @class
 */
export default class Subscription {
    
    /**
     * @param {Subscription} inner - nats connection subscription.
     * @constructor
     */
    constructor(inner) {
        this._inner = inner
        this._is_async = null
        this._codec = Nats.StringCodec()
    }
    
    /**
     * @description hooks message handler result.
     * @param {Promise<any> | any} result
     * @param {any}
     * @private
     */
    _hook(result) {
        if (this._is_async === null) {
            this._is_async = result instanceof Promise
        }
        
        return result
    }

    /**
     * @typedef {Object} Response
     * @property {string|null} error - response error description.
     * @property {any|null} data - response data struct.
     */
    /**
     * @description callback nats subscription message.
     * @param {Message} message - nats subscription message.
     * @param {any} payload - callback result.
     * @private
     */
    _callback(message, payload) {
        message.respond(this._codec.encode(
            JSON.stringify(payload)
        ))
    }
    
    /**
     * this callback is handler.
     * @callback Handler
     * @param {any} payload
     * @param {Message} message
     * @returns {Promise<any> | void}
     */
    /**
     * @description handle topic message.
     * @param {Subscription~Handler} handler
     * @returns {Promise<void>}
     * @public
     * @example
     * const mysticeti = new mystical({
     *     server: "localhost:4222"
     * })
     * 
     * mysticeti.Broker.auth.handler(message => {
     *     // console.log(message)
     * })
     */
    async handler(handler) {
        for await (const message of this._inner) { 
            let data = null
            let error = null
            
            try {
                const payload = this._codec.decode(message.data)
                const result = this._hook(handler(payload, message))
                data = this._is_async ? await result : result
            } catch(e) {
                error = e.message
            }
            
            this._callback(message, {
                error, 
                data 
            })
        }
    }
}
