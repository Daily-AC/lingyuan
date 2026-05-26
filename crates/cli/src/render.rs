pub fn render_markdown(obs: &serde_json::Value) -> String {
    let name = obs["self"]["name"].as_str().unwrap_or("?");
    let tick = obs["tick"].as_u64().unwrap_or(0);
    let clock = &obs["clock"];
    let status = &obs["self"]["status"];
    let mut s = String::new();
    s.push_str(&format!(
        "## You are {} — tick {}, {} 季 {} 日 {} 时\n\n",
        name,
        tick,
        clock["season"].as_str().unwrap_or("?"),
        clock["day"].as_u64().unwrap_or(0),
        clock["tick_in_day"].as_u64().unwrap_or(0),
    ));
    s.push_str(&format!(
        "**Status:** HP {}/100 · 饥 {}/100 · 力 {}/100 · 温 {} · 灵识 {}\n\n",
        status["hp"], status["hunger"], status["stamina"], status["warmth"], status["sanity"]
    ));
    s.push_str("**You see:**\n");
    let pos = &obs["self"]["pos"];
    s.push_str(&format!("- ({},{}) you\n", pos["x"], pos["y"]));
    if let Some(arr) = obs["visible_entities"].as_array() {
        for e in arr {
            if e["kind"] == "agent" {
                s.push_str(&format!(
                    "- ({},{}) **{}** [agent, HP {}]\n",
                    e["pos"]["x"], e["pos"]["y"], e["name"], e["hp"]
                ));
            }
        }
    }
    if let Some(tiles) = obs["vision"]["tiles"].as_array() {
        s.push_str(&format!("\n*({} tiles visible)*\n", tiles.len()));
    }

    if let Some(signs) = obs["nearby_signs"].as_array() {
        if !signs.is_empty() {
            s.push_str("\n**Signs nearby:**\n");
            for sg in signs {
                s.push_str(&format!(
                    "- ({},{}) \"{}\" — {}\n",
                    sg["pos"]["x"],
                    sg["pos"]["y"],
                    sg["text"].as_str().unwrap_or(""),
                    sg["author"].as_str().unwrap_or("anon")
                ));
            }
        }
    }

    if let Some(mail) = obs["mail"].as_array() {
        if !mail.is_empty() {
            s.push_str("\n**Mail:**\n");
            for m in mail {
                s.push_str(&format!(
                    "- {} → 你 (tick {}): {}\n",
                    m["from"].as_str().unwrap_or("?"),
                    m["received_at_tick"].as_u64().unwrap_or(0),
                    m["text"].as_str().unwrap_or("")
                ));
            }
        }
    }
    s
}
