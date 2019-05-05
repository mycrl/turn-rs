const net = require("net")


net.createServer(function (socket) {
    function rand (count) {
        let arrs = [];
        for (let i = 0; i < count; i ++) {
            arrs.push(Math.floor(Math.random() * 255))
        }
        return arrs
    }

    socket.on("data", bytes => {
        console.log("bytes", bytes);
        console.log("bytes len", bytes.length);

        let is_type = false;
        let is_back = false;
        let index = 0;
        let offset = 0;

        // examination package length.
        // C0 + C1 || S0 + S1
        if (bytes.length == 1537) {
            // C0, S0
            // lock version number is 3
            if (bytes[0] == 3) {
                index = 5;
                offset = 9;
                is_back = true;
            }
        } else {
            index = 4;
            offset = 8;
        }

        // C1, C2
        // S1, S2
        // TODO: check only the default placeholder.
        if (index > 0) {
            if (bytes[index] == 0 
            && bytes[index + 1] == 0 
            && bytes[index + 2] == 0 
            && bytes[index + 3] == 0) {
                if (bytes.length - offset == 1528) {
                    is_type = true
                }
            }
        }

        // callback type and back.
        if (is_type && is_back) {
            let body = [ 3 ];
            body.push(...[ 0, 0, 0, 0, 0, 0, 0, 0 ]);
            body.push(...rand(1528));
            body.push(...[ 0, 0, 0, 0, 0, 0, 0, 0 ]);
            body.push(...rand(1528));
            socket.write(Buffer.from(body));
        }
    })
}).listen(1935)