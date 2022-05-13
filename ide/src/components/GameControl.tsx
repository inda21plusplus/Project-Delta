import { ButtonGroup, Button } from "@mui/material";
import { invoke } from "@tauri-apps/api";
import { FunctionComponent, useState } from "react";
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import PauseIcon from '@mui/icons-material/Pause';
import StopIcon from '@mui/icons-material/Stop';

const GameControl: FunctionComponent = () => {
  const [play, setPlay] = useState(false);
  const [pause, setPause] = useState(false);

  return (<ButtonGroup aria-label="outlined primary button group">
    {play ? (
    <Button onClick={() => {
      (async () => {
        await invoke<void>('stop_game');
        setPlay(false);
      })()
    }} variant="contained"><StopIcon/></Button>
    ) : (
    <Button onClick={() => {
      (async () => {
        await invoke<void>('play_game');
        setPlay(true);
      })()
    }} variant="outlined"><PlayArrowIcon/></Button>
    )}
    {pause ? (
    <Button onClick={() => {
      (async () => {
        await invoke<void>('unpause_game');
        setPause(false);
      })()
    }} variant="contained"><PauseIcon/></Button>
    ) : (
    <Button onClick={() => {
      (async () => {
        await invoke<void>('pause_game');
        setPause(true);
      })()
    }} variant="outlined"><PauseIcon/></Button>
    )}
  </ButtonGroup>);
}

export default GameControl;
