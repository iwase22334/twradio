import * as React from "react";
import { createContext, ReactNode } from "react";

import { TweetProps } from "./components/TweetCard";
import { SpeakerInfo, toUniqueSpeakerId } from "./components/SettingsView";

import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";

type AppContextType = {
  focusTweetIdPair: [string, React.Dispatch<string>];
  tweetListPair: [Array<TweetProps>, React.Dispatch<Array<TweetProps>>];
  searchTweetListPair: [Array<TweetProps>, React.Dispatch<Array<TweetProps>>];
  skippedPair: [boolean, React.Dispatch<boolean>];
  pausedPair: [boolean, React.Dispatch<boolean>];
  focusedPair: [boolean, React.Dispatch<boolean>];
  speakerPair: [string, React.Dispatch<string>];
  speakerListPair: [Array<SpeakerInfo>, React.Dispatch<Array<SpeakerInfo>>];
  speechRatePair: [number, React.Dispatch<number>];
};

export const AppContext = createContext({} as AppContextType);

export const AppContextProvider = ({ children }: { children: ReactNode }) => {
  const [focusTwid, setFocusTwid] = React.useState<string>("");
  const [tweetList, setTweetList] = React.useState<Array<TweetProps>>([]);
  const [searchTweetList, setSearchTweetList] = React.useState<Array<TweetProps>>([]);
  const [skipped, setSkipped] = React.useState(false);
  const [paused, setPaused] = React.useState(false);
  const [focused, setFocused] = React.useState(true);

  const [speaker, setSpeaker] = React.useState(() => {
    console.log("speaker initialized");

    const json = localStorage.getItem("speaker");
    const parsedInitSpeaker = json === null ? null : JSON.parse(json);
    const initSpeaker =
      parsedInitSpeaker === null ? "127.0.0.1:50021/0" : parsedInitSpeaker;

    return initSpeaker;
  });

  const [speakerList, setSpeakerList] = React.useState<Array<SpeakerInfo>>([]);

  React.useEffect(() => {
    console.log("speaker listener steart");

    listen<Array<SpeakerInfo>>(
      "tauri://frontend/speakers-register",
      (event) => {
        const speakers: Array<SpeakerInfo> = event.payload;
        console.log(speakers);

        speakerList.splice(0);
        for (let sp of speakers) {
          speakerList.push({
            addr: sp.addr,
            engine: sp.engine,
            name: sp.name,
            style: sp.style,
            speaker: sp.speaker,
          });
        }

        const index = speakerList.findIndex(
          (e) => toUniqueSpeakerId(e) === speaker
        );
        invoke("set_speaker", { speaker: speakerList[index] });

        setSpeakerList([...speakerList]);
      }
    );
  }, []);

  const [speechRate, setSpeechRate] = React.useState(() => {
    const json = localStorage.getItem("speechRate");
    const parsedInitSpeechRate = json === null ? null : JSON.parse(json);
    const initSpeechRate =
      parsedInitSpeechRate === null ? 1.0 : parsedInitSpeechRate;

    return initSpeechRate;
  });

  React.useEffect(() => {
    console.log("speechRate" + speechRate);
    invoke("set_speech_rate", { speechRate });
    localStorage.setItem("speechRate", JSON.stringify(speechRate as number));
  }, [speechRate]);

  return (
    <AppContext.Provider
      value={{
        focusTweetIdPair: [focusTwid, setFocusTwid],
        tweetListPair: [tweetList, setTweetList],
        searchTweetListPair: [searchTweetList, setSearchTweetList],
        skippedPair: [skipped, setSkipped],
        pausedPair: [paused, setPaused],
        focusedPair: [focused, setFocused],
        speakerPair: [speaker, setSpeaker],
        speakerListPair: [speakerList, setSpeakerList],
        speechRatePair: [speechRate, setSpeechRate],
      }}
    >
      {children}
    </AppContext.Provider>
  );
};
