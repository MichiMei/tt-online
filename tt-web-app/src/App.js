
import React, { Component } from 'react';
import { w3cwebsocket as W3CWebSocket } from "websocket";

const client = new W3CWebSocket('ws://127.0.0.1:8080');

class App extends Component {

  sendLogin() {
    if (client.readyState === client.OPEN) {
      // TODO get real data
      const name = "mock_name"
      const type = "ClientLogin"
      const message_obj = {type:type, name: name}
      const message_str = JSON.stringify(message_obj)

      console.log("sendLogin(..): " + message_str)
      client.send(message_str)
    }
  }

  sendHello() {
    if (client.readyState === client.OPEN) {
      console.log("sent: Hello World!")
      client.send("{\"type\":\"ClientLogin\",\"name\":\"test_name\"}");
    }
  }

  componentDidMount() {
    client.onopen = () => {
      console.log('componentDidMount(..): WebSocket Client Connected');
      this.sendLogin()
    };
    client.onmessage = (message) => {
      console.log("received: " + message.data.toString());
    };
  }

  render() {
    return (
        <div>
          Practical Intro To WebSockets.
        </div>
    );
  }
}

export default App;

/*
import logo from './logo.svg';
import './App.css';

function App() {
  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.js</code> and save to reload.
        </p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
}

export default App;
*/