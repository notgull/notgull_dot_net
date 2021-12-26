// GNU AGPL v3 License

import React, { Component } from "react";

import { Empty } from "./util";

interface LoadingState {
    dots: number,
    interval: ReturnType<typeof setInterval> | undefined,
}

export class Loading extends Component<Empty, LoadingState> {
    state = {
        dots: 1,
        interval: undefined,
    };

    componentDidMount() {
        const interval = setInterval(() => {
            // increase dots count by 1, clamped to 3
            let dots = this.state.dots;
            dots += 1;
            if (dots > 3) { dots = 1; }

            this.setState({ dots });
        }, 100);

        this.setState({ interval });
    }

    componentWillUnmount() {
        if (this.state.interval !== undefined) {
            clearInterval(this.state.interval);
        }
    }

    render() {
        let dots = "";
        for (let i = 0; i < this.state.dots; i++) {
            dots += ".";
        }

        return (
            <p>
                Loading{dots}
            </p>
        )
    }
};

export default Loading;