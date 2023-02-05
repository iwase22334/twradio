use serde::{Deserialize, Serialize};
use std::collections::LinkedList;

use crate::audio_player;
use crate::display_bridge;
use crate::twitter_data;
use crate::user_input;
use crate::voicegen_agent;

const HISTORY_LENGTH: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub tweet_id: String,
    pub created_at: String,
    pub text: String,
    pub name: String,
    pub username: String,
    pub profile_image_url: String,
}

pub fn into(tweet: &twitter_data::Tweet, users: &Vec<twitter_data::User>) -> Record {
    let user = users
        .iter()
        .find(|user| user.id == tweet.author_id)
        .unwrap();

    Record {
        tweet_id: tweet.id.clone(),
        created_at: tweet.created_at.clone(),
        text: tweet.text.clone(),
        name: user.name.clone(),
        username: user.username.clone(),
        profile_image_url: user.profile_image_url.clone(),
    }
}

fn remove<T>(list: &mut LinkedList<T>, index: usize) -> T {
    if index == 0 {
        let v = list.pop_front().unwrap();

        return v;
    } else {
        // split_off function should compute in O(n) time.
        let mut split = list.split_off(index);
        let v = split.pop_front().unwrap();
        list.append(&mut split);

        return v;
    }
}

struct Context {
    pub addr: std::net::SocketAddr,
    pub speaker: u64,
    pub speech_rate: f64,
    pub focus_set: bool,
    pub cancelling: bool,
    pub paused: bool,
    pub tts_processing: bool,
    pub wait_list: LinkedList<Record>,
    pub ready_list: LinkedList<Record>,
    pub played_list: LinkedList<Record>,
    pub speech_cache: LinkedList<voicegen_agent::Speech>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], 50031)),
            speaker: 0,
            speech_rate: 1.0f64,
            focus_set: false,
            cancelling: false,
            paused: false,
            tts_processing: false,
            wait_list: LinkedList::<Record>::new(),
            ready_list: LinkedList::<Record>::new(),
            played_list: LinkedList::<Record>::new(),
            speech_cache: LinkedList::<voicegen_agent::Speech>::new(),
        }
    }
}

fn remove_cache(ctx: &mut Context) {
    if ctx.tts_processing {
        ctx.tts_processing = false;
        ctx.cancelling = true;
    }
    if ctx.ready_list.len() > 0 {
        ctx.ready_list.append(&mut ctx.wait_list);
        ctx.wait_list = ctx.ready_list.split_off(0);
        ctx.speech_cache.split_off(0);
    }
}

pub fn start(
    _app_handle: tauri::AppHandle,
    mut tweet_rx: tokio::sync::mpsc::Receiver<Record>,
    display_tx: tokio::sync::mpsc::Sender<display_bridge::DisplayContrl>,
    playbook_tx: tokio::sync::mpsc::Sender<voicegen_agent::Playbook>,
    mut speech_rx: tokio::sync::mpsc::Receiver<Option<voicegen_agent::Speech>>,
    audioctl_tx: tokio::sync::mpsc::Sender<audio_player::AudioControl>,
    mut audioctl_rdy_rx: tokio::sync::mpsc::Receiver<audio_player::AudioControlRdy>,
    mut user_rx: tokio::sync::mpsc::Receiver<user_input::UserInput>,
) {
    // Context
    let mut ctx = Context::new();

    // Operating clock
    let (clk_tx, mut clk_rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            let _ = clk_tx.send(()).await;
        }
    });

    tokio::spawn(async move {
        loop {
            println!(
                "{:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
                ctx.addr,
                ctx.speaker,
                ctx.speech_rate,
                ctx.wait_list.len(),
                ctx.ready_list.len(),
                ctx.played_list.len(),
                ctx.speech_cache.len(),
                ctx.cancelling,
                ctx.paused,
                ctx.tts_processing
            );

            print!("scheduler: Select> ");
            tokio::select! {
                Some(_) = clk_rx.recv() => {
                    // Obtain Tweet

                    if !ctx.paused {
                        match tweet_rx.try_recv() {
                            Ok(msg) => {
                                println!("New tweet incoming {:?}", msg.tweet_id);

                                ctx.wait_list.push_back(msg.clone());
                                display_tx.send(display_bridge::DisplayContrl::Add(msg.clone().into())).await.unwrap();

                                if !ctx.focus_set {
                                    display_tx.send(display_bridge::DisplayContrl::Scroll(msg.tweet_id)).await.unwrap();
                                    ctx.focus_set = true;
                                }
                            }

                            Err(e) => {
                                match e {
                                    tokio::sync::mpsc::error::TryRecvError::Empty => {},

                                    e => {
                                        println!("scheduler: twitter agent closes pci {:?}", e);
                                        return ();
                                    }
                                }
                            },
                        }
                    }

                    // TTS Start
                    if ctx.wait_list.len() > 0
                        && !ctx.tts_processing
                        && !ctx.cancelling {

                        ctx.tts_processing = true;
                        playbook_tx.send(
                            voicegen_agent::into(ctx.wait_list.front().unwrap().clone().into(), ctx.addr, ctx.speaker, ctx.speech_rate)).await.unwrap();

                        println!("<clk>start tts_processing {:?}", ctx.wait_list.front().as_ref().unwrap().tweet_id);
                    }

                    // Process TTS Result
                    match speech_rx.try_recv() {
                        Ok(speech) => {
                            if ctx.cancelling {
                                // Ignore processing result.
                                println!("tts result is ignored");
                                ctx.cancelling = false;
                                continue;
                            }

                            match speech {
                                Some(speech) => {
                                    println!("Text to speech is complete {:?}", speech.tweet_id);

                                    ctx.tts_processing = false;
                                    ctx.ready_list.push_back(ctx.wait_list.pop_front().unwrap());
                                    ctx.speech_cache.push_back(speech);
                                }
                                None => {
                                    println!("Text to speech is failed ");
                                    ctx.tts_processing = false;
                                }
                            }
                        },

                        Err(e) => {
                            match e {
                                tokio::sync::mpsc::error::TryRecvError::Empty => {},

                                e => {
                                    println!("scheduler: voicegen_agent closes pci {:?}", e);
                                    return ();
                                }
                            }
                        },
                    }

                    // Play speech
                    if ctx.ready_list.len() > 0 {
                        match audioctl_rdy_rx.try_recv() {
                            Ok(_) => {
                                println!("Audio and speech is ready, start playing.");

                                let target_tw = ctx.ready_list.pop_front().unwrap();
                                let target_twid = target_tw.tweet_id.clone();

                                let index = ctx.speech_cache.iter().position(|x| x.tweet_id == target_twid).unwrap();
                                let s = remove(&mut ctx.speech_cache, index);

                                let voice_pack = vec![s.name, s.text];
                                audioctl_tx.send(audio_player::AudioControl::PlayMulti(voice_pack)).await.unwrap();

                                ctx.played_list.push_back(target_tw);
                                if ctx.played_list.len() > HISTORY_LENGTH {
                                    let ve = ctx.played_list.pop_front().unwrap();
                                    display_tx.send(display_bridge::DisplayContrl::Delete(ve.tweet_id)).await.unwrap();
                                }
                                display_tx.send(display_bridge::DisplayContrl::Scroll(target_twid)).await.unwrap();
                            },

                            Err(e) => {
                                match e {
                                    tokio::sync::mpsc::error::TryRecvError::Empty => {},

                                    e => {
                                        println!("scheduler: audio player closes pci {:?}", e);
                                        return ();
                                    }
                                }
                            },
                        }
                    }

                }

                Some(user) = user_rx.recv() => {
                    print!("User input - ");
                    match user {
                        user_input::UserInput::Jump(twid) => {
                            print!("jump to {:?}", twid);
                            audioctl_tx.send(audio_player::AudioControl::Stop).await.unwrap();

                            // Cancel current playing speech only;
                            if twid == "" { continue; }

                            if ctx.tts_processing {
                                ctx.cancelling = true;
                            }

                            let p = ctx.wait_list.iter().position(|x| x.tweet_id == twid);
                            if p.is_some() {
                                ctx.played_list.append(&mut ctx.ready_list);

                                let tail = ctx.wait_list.split_off(p.unwrap());

                                ctx.played_list.append(&mut ctx.wait_list);
                                ctx.wait_list = tail;

                                ctx.tts_processing = false;
                                ctx.speech_cache.clear();
                            }

                            let p = ctx.ready_list.iter().position(|x| x.tweet_id == twid);
                            if p.is_some() {
                                let tail = ctx.ready_list.split_off(p.unwrap());

                                ctx.played_list.append(&mut ctx.ready_list);

                                ctx.ready_list = tail;

                                ctx.tts_processing = false;
                                let tail = ctx.speech_cache.split_off(p.unwrap());
                                ctx.speech_cache = tail;
                            }

                            while ctx.played_list.len() >= HISTORY_LENGTH {
                                let ve = ctx.played_list.pop_front().unwrap();
                                display_tx.send(display_bridge::DisplayContrl::Delete(ve.tweet_id)).await.unwrap();
                            }
                            display_tx.send(display_bridge::DisplayContrl::Scroll(twid)).await.unwrap();
                        },

                        user_input::UserInput::Paused(msg) => {
                            ctx.paused = msg;
                        }

                        user_input::UserInput::Speaker(speaker) => {
                            println!("{:?}", speaker);
                            ctx.addr = speaker.addr;
                            ctx.speaker = speaker.speaker;

                            remove_cache(&mut ctx);
                        }

                        user_input::UserInput::SpeechRate(speech_rate) => {
                            ctx.speech_rate = speech_rate;

                            remove_cache(&mut ctx);
                        }
                    }
                }

                else => {
                    println!("Core thread exit");
                    return ();
                }
            }
        }
    });
}
