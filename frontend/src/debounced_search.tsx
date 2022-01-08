// GNU AGPL v3 License

import { Component, h } from "preact";

interface DebouncedSearchProps {
    onChange: (search: string) => void,
    debounceTime: number,
    startingSearch: string,
};

interface DebouncedSearchState {
    search: string,
    timeout: ReturnType<typeof setTimeout> | undefined,
};

export class DebouncedSearch extends Component<DebouncedSearchProps, DebouncedSearchState> {
    state = {
        search: "",
        timeout: undefined,
    };

    componentDidMount() {
        this.setState({ search: this.props.startingSearch });
    }

    doTextChange(search: string) {
        if (this.state.timeout !== undefined) {
            clearTimeout(this.state.timeout);
        }

        this.setState({
            search,
            timeout: setTimeout(() => {
                this.props.onChange(this.state.search);
            }, this.props.debounceTime),
        });
    }

    render() {
        return (
            <input 
                type="text" 
                value={this.state.search} 
                onChange={(ev) => this.doTextChange((ev!.target! as HTMLInputElement).value)} />
        );
    }
};

export default DebouncedSearch;