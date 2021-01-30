"use strict"

const fs = require("fs")
const path = require("path")
const exec = require("child_process").exec

function match_all(source) {
    return source
        .split(
            "\u0060\u0060\u0060" +
            "\u0074\u0065\u0073\u0074"
        ).slice(1)
        .filter(x => x.search("\u0060\u0060\u0060") >= 0)
        .map(x => x.split("\u0060\u0060\u0060")[0])
}

function split_test(source) {
    let body = source.replace(
        /\u002F\u002F\u002F/g, 
        "\u0020\u0020\u0020"
    )
    
    let name = body.split("(")[1].split(")")[0]
    let test = body.slice(name.length + 2)
    return { 
        name, 
        test 
    }
}

function stringify({ test, name }) {
    let target = ""
    let is_async = test.search(".await") >= 0
    target += `    #[${is_async ? "tokio::" : ""}test]\r\n`
    target += `    ${is_async ? "async " : ""}fn `
    target += name
    target += "() {\r\n"
    target += test
    target += "\r\n    }\r\n\r\n"
    return target
}

function remux(source, handle) {
    let target = ""
    let tests = []
    
    if (source.search(
        "\u002F\u002F\u002F" +
        "\u0020" +
        "\u0060\u0060\u0060" +
        "\u0074\u0065\u0073\u0074"
    ) < 0) {
        return undefined
    }
    
    match_all(source).forEach(item => {
        tests.push(split_test(item))
    })
    
    target += "\r\n\r\n"
    target += "#[cfg(test)]\r\n"
    target += "mod tests {\r\n"
    target += tests.map(stringify).join("")
    target += "}\r\n"
    
    handle(
        source, 
        target
    )
}

function reader(branch, emit) {
    fs.readdirSync(branch)
        .map(x => path.join(branch, x))
        .forEach(node => {
            node.endsWith(".rs") ? 
                emit(node) : 
                reader(node, emit)
        })
}

let SourceBufs = []

reader(path.resolve("../src"), branch => {
    remux(fs.readFileSync(branch, "utf8"), (buf, test) => {
        fs.writeFileSync(branch, buf + test)
        SourceBufs.push({buf, branch})
    })
})

let pcs = exec("cargo test", { 
    cwd: path.resolve("../") 
})

pcs.stderr.pipe(process.stderr)
pcs.stdout.pipe(process.stdout)

pcs.on("exit", () => {
    SourceBufs.forEach(({buf, branch}) => {
        fs.writeFileSync(branch, buf)
    })
})