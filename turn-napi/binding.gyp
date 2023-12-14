{
    "targets": [
        {
            "target_name": "turn",
            "defines": [ 
                "NAPI_DISABLE_CPP_EXCEPTIONS"
            ],
            "sources": [
                "src/turn.cpp"
            ],
            "include_dirs": [
                "<!@(node -p \"require('node-addon-api').include\")",
                "../turn-exports"
            ],
            "libraries": [
                "../../target/debug/libturn.dll.lib"
            ],
            "cflags_cc": [
                "-std=c++17"
            ],
            "cflags!": [
                "-fno-exceptions"
            ],
            "cflags_cc!": [
                "-fno-exceptions"
            ]
        }
    ]
}
