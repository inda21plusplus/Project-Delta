import { useEffect, useState } from 'react';
import './App.scss';
import { invoke } from '@tauri-apps/api';
import { Button, ButtonGroup } from '@mui/material';
import Header from './components/Header';
import Main from './views/Main';

function App() {
  return (
    <div className="App">
      <Header/>
      <Main/>
      {/* <div id="wasm-example"/> */}
    </div>
  );
}

export default App;
