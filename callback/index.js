const callback = require('./callback.node')

callback.callThreadsafeFunction((err, ...args) => {
    console.log(args)
    return 1
})
