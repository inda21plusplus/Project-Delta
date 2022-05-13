import { Paper } from "@mui/material";
import { FunctionComponent } from "react";
import GameControl from "../components/GameControl";
import ObjectList from "../components/ObjectList";
import "./Main.scss";

const Main: FunctionComponent = () => {
  const console = ["text", "asd"];

  return (<div className="Main">
    <div>
      <Paper component="aside">
        <ObjectList/>
      </Paper>
      <Paper component="main">
        <div className="Game">
          <canvas/>
          <GameControl/>
        </div>
      </Paper>
    </div>
    <Paper component="footer">
      <ul>{console.map(line =>
        <li>
          {line}
        </li>
      )}</ul>
    </Paper>
  </div>);
}

export default Main;
