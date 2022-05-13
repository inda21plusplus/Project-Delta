import { Button } from "@mui/material";
import { invoke } from "@tauri-apps/api";
import { FunctionComponent, useState } from "react";
import "./Header.scss";

const Header: FunctionComponent = () => {
  return (<header className="Header">
    <Button
      onClick={() => {
        (async () => {
          const response = await invoke<void>('select_game');
          console.log(response);
        })()
      }}
    >Select folder</Button>
  </header>);
}

export default Header;
