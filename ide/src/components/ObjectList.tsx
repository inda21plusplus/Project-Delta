import { Button } from "@mui/material";
import { invoke } from "@tauri-apps/api";
import { FunctionComponent, useState } from "react";

const ObjectList: FunctionComponent = () => {
  const objects = ["text", "asd"];

  return (<ul>{objects.map(line =>
    <li>
      {line}
    </li>
  )}</ul>);
}

export default ObjectList;
