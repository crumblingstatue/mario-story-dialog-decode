use {
    hexerator_plugin_api::{
        HexeratorHandle, MethodParam, MethodResult, Plugin, PluginMethod, Value, ValueTy,
    },
    mario_story_dialog_decode::{decode_imm_buf, BUFFER_SIZE},
};

struct MarioStoryPlugin;

impl Plugin for MarioStoryPlugin {
    fn name(&self) -> &str {
        "mario-story-dialog"
    }

    fn desc(&self) -> &str {
        "Decoder for Mario Story (Japanese Paper Mario) dialog"
    }

    fn methods(&self) -> Vec<hexerator_plugin_api::PluginMethod> {
        vec![
            PluginMethod {
                method_name: "decode_range",
                human_name: None,
                desc: "Decodes a range of bytes as paper mario dialogue",
                params: &[
                    MethodParam {
                        name: "from",
                        ty: ValueTy::U64,
                    },
                    MethodParam {
                        name: "to",
                        ty: ValueTy::U64,
                    },
                ],
            },
            PluginMethod {
                method_name: "decode_range_nth_bubble",
                human_name: None,
                desc: "Decodes a range of bytes as paper mario dialogue (nth bubble)",
                params: &[
                    MethodParam {
                        name: "from",
                        ty: ValueTy::U64,
                    },
                    MethodParam {
                        name: "to",
                        ty: ValueTy::U64,
                    },
                    MethodParam {
                        name: "bubble",
                        ty: ValueTy::U64,
                    },
                ],
            },
            PluginMethod {
                method_name: "decode_selection",
                human_name: None,
                desc: "Decodes hexerator selection as paper mario dialogue",
                params: &[],
            },
            PluginMethod {
                method_name: "decode_imm_buf",
                human_name: Some("Decode immediate buffer"),
                desc: "Decodes the immediate dialog buffer",
                params: &[
                    MethodParam {
                        name: "offset",
                        ty: ValueTy::U64,
                    },
                    MethodParam {
                        name: "scroll",
                        ty: ValueTy::U64,
                    },
                ],
            },
        ]
    }

    fn on_method_called(
        &mut self,
        name: &str,
        params: &[Option<Value>],
        hexerator: &mut dyn HexeratorHandle,
    ) -> MethodResult {
        match name {
            "decode_imm_buf" => {
                let &[Some(Value::U64(offset)), Some(Value::U64(scroll))] = params else {
                    return Err("Invalid arguments".into());
                };
                match hexerator.get_data(offset as usize, offset as usize + BUFFER_SIZE) {
                    Some(data) => Ok(Some(Value::String(
                        decode_imm_buf(data, scroll as u32).text,
                    ))),
                    None => Err("out of bounds".into()),
                }
            }
            "decode_range" => {
                let &[Some(Value::U64(from)), Some(Value::U64(to))] = params else {
                    return Err("Invalid arguments".into());
                };
                match hexerator.get_data(from as usize, to as usize) {
                    Some(data) => match mario_story_dialog_decode::to_string(data) {
                        Ok(string) => Ok(Some(Value::String(string))),
                        Err(e) => Err(e.to_string()),
                    },
                    None => Err("Range out of bounds".into()),
                }
            }
            "decode_range_nth_bubble" => {
                let &[Some(Value::U64(from)), Some(Value::U64(to)), Some(Value::U64(bubble))] =
                    params
                else {
                    return Err("Invalid arguments".into());
                };
                match hexerator.get_data(from as usize, to as usize) {
                    Some(data) => {
                        match mario_story_dialog_decode::to_string_nth_bubble(data, bubble as u8) {
                            Ok(string) => Ok(Some(Value::String(string))),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    None => Err("Range out of bounds".into()),
                }
            }
            "decode_selection" => {
                let Some((from, to)) = hexerator.selection_range() else {
                    return Err("No selection".into());
                };
                match hexerator.get_data(from, to) {
                    Some(data) => match mario_story_dialog_decode::to_string(data) {
                        Ok(string) => Ok(Some(Value::String(string))),
                        Err(e) => Err(e.to_string()),
                    },
                    None => Err("Range out of bounds".into()),
                }
            }
            _ => Err(format!("Unknown method: {name}")),
        }
    }
}

#[no_mangle]
pub extern "Rust" fn hexerator_plugin_new() -> Box<dyn Plugin> {
    Box::new(MarioStoryPlugin)
}
