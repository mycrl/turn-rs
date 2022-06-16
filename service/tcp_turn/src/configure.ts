/**
 * configure
 * 
 * @module
 */

export interface Configure {
    port: number
    proxy_ip: string
    proxy_port: number
}

/**
 * env configure
 * 
 * @readonly
 */
export const Config: Configure = {
    port: Number(process.env.TCP_TURN_PORT || '3478'),
    proxy_ip: process.env.TCP_TURN_PROXYIP || '127.0.0.1',
    proxy_port: Number(process.env.TCP_TURN_PROXYPORT || '3478'),
}
