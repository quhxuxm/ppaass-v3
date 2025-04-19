export class UserInfo {
    constructor(nickname: string, proxyAddress: string, username: string) {
        this._nickname = nickname;
        this._proxyAddress = proxyAddress;
        this._username = username;
    }

    private _nickname?: string;
    get nickname(): string | undefined {
        return this._nickname;
    }

    set nickname(value: string) {
        this._nickname = value;
    }

    private _proxyAddress: string;
    get proxyAddress(): string {
        return this._proxyAddress;
    }

    set proxyAddress(value: string) {
        this._proxyAddress = value;
    }

    private _username: string;
    get username(): string {
        return this._username;
    }

    set username(value: string) {
        this._username = value;
    }
}