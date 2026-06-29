import { defineConfig } from "vitepress";

// turn-rs is a project (not user/org) GitHub Pages site, so it is served
// from https://<owner>.github.io/turn-rs/ and needs a matching base path.
export default defineConfig({
    base: "/turn-rs/",
    title: "turn-rs",
    description:
        "A high-performance TURN/STUN server implemented in Rust for WebRTC NAT traversal and media relay.",
    lang: "en-US",
    lastUpdated: true,
    cleanUrls: true,

    themeConfig: {
        nav: [
            { text: "Home", link: "/" },
            { text: "Guide", link: "/guide/install" },
            { text: "Configure", link: "/guide/configure" },
            { text: "Demo", link: "/demo" },
            { text: "GitHub", link: "https://github.com/mycrl/turn-rs" },
        ],

        // The actual document content lives in the repository's top-level `../docs`
        // folder. The pages under `guide/` are thin stubs that include that content
        // at build time, so `docs/` stays the single source of truth and nothing is
        // duplicated into the website subproject.
        sidebar: [
            {
                text: "Documentation",
                items: [
                    { text: "Install", link: "/guide/install" },
                    { text: "Build", link: "/guide/build" },
                    { text: "Start the Server", link: "/guide/start-the-server" },
                    { text: "Configure", link: "/guide/configure" },
                    {
                        text: "Migrating from coturn",
                        link: "/guide/migrate-from-coturn",
                    },
                ],
            },
            {
                text: "Examples",
                items: [{ text: "WebRTC Demo", link: "/demo" }],
            },
        ],

        socialLinks: [{ icon: "github", link: "https://github.com/mycrl/turn-rs" }],

        search: {
            provider: "local",
        },

        footer: {
            message: "Released under the MIT License.",
            copyright: "Copyright © 2022 Mycrl.",
        },
    },
});
