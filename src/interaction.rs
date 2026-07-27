use crate::ui::window::{ActiveProp, PropId};

#[derive(Debug, Clone)]
pub struct AppParticleConfig {
    pub spawn_chance: f64,
    pub spawn_box: (f64, f64, f64, f64), // (x, y, width, height)
    pub size_range: (f64, f64),
    pub speed_x_range: (f64, f64),
    pub speed_y_range: (f64, f64),
    pub range: Vec<PropId>,
}

#[derive(Debug, Clone)]
pub struct AppInteraction {
    pub match_patterns: Vec<&'static str>,
    pub prop: ActiveProp,
    pub focus_comments: Vec<&'static str>,
    pub periodic_comments: Vec<&'static str>,
    pub particle_config: AppParticleConfig,
}

pub fn get_app_interactions() -> Vec<AppInteraction> {
    vec![
        AppInteraction {
            match_patterns: vec!["wuthering", "waves"],
            prop: ActiveProp::WutheringWaves,
            focus_comments: vec![
                "Absorbing Echoes! 🌌",
                "Rover, look out! ⚔️",
                "Time to farm echo substats... 😭",
                "Defeating the Crownless! ⚔️",
            ],
            periodic_comments: vec![
                "Checking Echo stats... Max Crit Rate? 🤔",
                "Rover, let's complete our daily commissions! 📋",
                "This Tacet Field is active! ⚡",
                "Listen to the sound of waves... 🌊",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.25,
                spawn_box: (167.5, 67.5, 35.0, 35.0),
                size_range: (4.0, 10.0),
                speed_x_range: (-0.3, 0.3),
                speed_y_range: (-1.1, -0.3),
                range: vec![PropId::Spark(false), PropId::Spark(true)],
            },
        },
        AppInteraction {
            match_patterns: vec![
                "reverse: 1999",
                "reverse 1999",
                "reverse1999",
                "reverse:1999",
            ],
            prop: ActiveProp::Reverse1999,
            focus_comments: vec![
                "The Storm is coming! 🌧️",
                "Time keeps moving backwards...",
                "Would you like some tea, Vertin? ☕",
                "Regulus is spinning records! 📻",
            ],
            periodic_comments: vec![
                "Is the rain falling up? 🌧️",
                "Vertin, Vertin! Look at the spinning pocket watch! 🕰️",
                "Let's brew some black tea. ☕",
                "Keep moving, don't get caught in the Storm! 🌪️",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.35,
                spawn_box: (0.0, 250.0, 256.0, 1.0),
                size_range: (10.0, 25.0),
                speed_x_range: (0.0, 0.0),
                speed_y_range: (-8.0, -4.0),
                range: vec![PropId::Rain],
            },
        },
        AppInteraction {
            match_patterns: vec!["sublime", "kitty"],
            prop: ActiveProp::SublimeKitty,
            focus_comments: vec![
                "Compile successful! 🚀",
                "Fixing bugs... 🐛",
                "Writing some clean Rust code! 🦀",
                "cat /dev/urandom 🐱",
            ],
            periodic_comments: vec![
                "Code compiles cleanly! Crab power! 🦀",
                "Compiling... perfect time for a quick pet? 🥰",
                "git commit -m 'pet the kitty' 🐾",
                "Blinking terminal cursor is soothing... 💻",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.15,
                spawn_box: (100.0, 150.0, 140.0, 50.0),
                size_range: (9.0, 13.0),
                speed_x_range: (-0.8, -0.3),
                speed_y_range: (-1.2, -0.5),
                range: vec![
                    PropId::Code(0),
                    PropId::Code(1),
                    PropId::Code(2),
                    PropId::Code(3),
                    PropId::Code(4),
                    PropId::Code(5),
                    PropId::Code(6),
                    PropId::Code(7),
                ],
            },
        },
        AppInteraction {
            match_patterns: vec!["code", "codium", "visual studio"],
            prop: ActiveProp::VSCode,
            focus_comments: vec![
                "VS Code is active! Let's write some code! 💻",
                "Tab or spaces? Let's use spaces! ⌨️",
                "No compiler errors, right? 🛠️",
                "Time to implement a new feature! ✨",
            ],
            periodic_comments: vec![
                "Checking Git diff... looks clean! 🌿",
                "Cargo check passed! Crab power! 🦀",
                "Remember to format with rustfmt! ⚙️",
                "Writing more unit tests... 🧪",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.20,
                spawn_box: (170.0, 70.0, 30.0, 30.0),
                size_range: (3.0, 7.0),
                speed_x_range: (-0.2, 0.2),
                speed_y_range: (-1.3, -0.5),
                range: vec![],
            },
        },
        AppInteraction {
            match_patterns: vec!["firefox", "chrome", "brave", "chromium", "zen"],
            prop: ActiveProp::Browser,
            focus_comments: vec![
                "Reading docs... or watching cat videos? 😸",
                "So many open tabs! 📑",
                "Surfing the web... 🌐",
                "Searching StackOverflow? 🔍",
            ],
            periodic_comments: vec![
                "Did you find the solution on GitHub? 🐙",
                "Yet another browser tab... 🌐",
                "Watching YouTube tutorials? 📺",
                "Let's check the weather forecast. ☀️",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.20,
                spawn_box: (167.5, 67.5, 35.0, 35.0),
                size_range: (8.0, 14.0),
                speed_x_range: (-0.25, 0.25),
                speed_y_range: (-1.0, -0.4),
                range: vec![],
            },
        },
        AppInteraction {
            match_patterns: vec!["discord"],
            prop: ActiveProp::Discord,
            focus_comments: vec![
                "Chatting with friends? 💬",
                "Discord ping! 🔔",
                "Ping @everyone... just kidding! 🤭",
                "Sharing memes? 😹",
            ],
            periodic_comments: vec![
                "Who's online? 🟢",
                "Sending a cute cat sticker... 🐾",
                "Joined a voice channel? 🎙️",
                "Keep the conversation flowing! 💬",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.15,
                spawn_box: (170.0, 70.0, 30.0, 30.0),
                size_range: (4.0, 9.0),
                speed_x_range: (-0.2, 0.2),
                speed_y_range: (-1.2, -0.6),
                range: vec![PropId::Spark(false), PropId::Spark(true)],
            },
        },
        AppInteraction {
            match_patterns: vec!["minecraft"],
            prop: ActiveProp::Minecraft,
            focus_comments: vec![
                "Mining diamonds... 💎",
                "Watch out for Creepers! 💥",
                "Sssss... BOOM! 💣",
                "Building a beautiful castle! 🧱",
            ],
            periodic_comments: vec![
                "Crafting more torches... 🕯️",
                "Hear that zombie? 🧟",
                "Time to feed the wolves! 🐺",
                "Entering the Nether... 🔥",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.25,
                spawn_box: (170.0, 85.0, 30.0, 5.0),
                size_range: (2.0, 5.0),
                speed_x_range: (-0.5, 0.5),
                speed_y_range: (0.5, 1.5),
                range: vec![PropId::Spark(false), PropId::Spark(true)],
            },
        },
        AppInteraction {
            match_patterns: vec!["steam"],
            prop: ActiveProp::Steam,
            focus_comments: vec![
                "Steam is open! Ready to launch a game? 🎮",
                "Checking friends list... 👥",
                "Any new games on sale? 💸",
            ],
            periodic_comments: vec![
                "Browsing the Steam Store... 🛍️",
                "Updating game library... 🔄",
                "Downloading latest updates... 📥",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.22,
                spawn_box: (170.0, 70.0, 30.0, 30.0),
                size_range: (3.0, 7.0),
                speed_x_range: (-0.3, 0.3),
                speed_y_range: (-1.5, -0.8),
                range: vec![PropId::Spark(false), PropId::Spark(true)],
            },
        },
        AppInteraction {
            match_patterns: vec!["spotify"],
            prop: ActiveProp::Spotify,
            focus_comments: vec![
                "Spotify is open! What's the vibe today? 🎵",
                "Discover Weekly time! 🎧",
                "Let the music play! 🎶",
            ],
            periodic_comments: vec![
                "Humming along to the beat... 🎤",
                "Adding this song to my favorites! ❤️",
                "Searching for the perfect playlist... 🔍",
            ],
            particle_config: AppParticleConfig {
                spawn_chance: 0.18,
                spawn_box: (170.0, 70.0, 30.0, 30.0),
                size_range: (10.0, 15.0),
                speed_x_range: (-0.4, 0.4),
                speed_y_range: (-1.0, -0.5),
                range: vec![PropId::MusicNote],
            },
        },
    ]
}
