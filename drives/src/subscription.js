"use strict"

/** 
 * @module Mysticeti/Subscription
 */

/**
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
    }
    
    /**
     * @description hooks message handler result.
     * @param {Promise | any} result
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
     * this callback is handler.
     * @callback Handler
     * @param {any} message
     * @returns {Promise | void}
     */
    /**
     * @description handle topic message.
     * @param {Subscription~Handler} handler
     * @returns {Promise}
     * @public
     * @example
     * const mysticeti = new Mysticeti({
     *     server: "localhost:4222"
     * })
     * 
     * mysticeti.Broker.auth.handler(message => {
     *     // console.log(message)
     * })
     */
    async handler(handler) {
        for await (const message of this._inner) { 
            try {
                const result = this._hook(handler(message))
                return this._is_async ? await result : result
            } catch(e) {
                console.warn(e)
            }
        }
    }
}