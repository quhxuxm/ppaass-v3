import {LogLevel} from "./common.ts";

export class DownloadSpeedInfoEvent {
    constructor(bytesPerSecond: number) {
        this._bytesPerSecond = bytesPerSecond;
    }

    private _bytesPerSecond: number
    get bytesPerSecond(): number {
        return this._bytesPerSecond;
    }

    set bytesPerSecond(value: number) {
        this._bytesPerSecond = value;
    }
}

export class UploadSpeedInfoEvent {
    constructor(bytesPerSecond: number) {
        this._bytesPerSecond = bytesPerSecond;
    }

    private _bytesPerSecond: number
    get bytesPerSecond(): number {
        return this._bytesPerSecond;
    }

    set bytesPerSecond(value: number) {
        this._bytesPerSecond = value;
    }
}

export class LogInfoEvent {
    constructor(level: LogLevel, message: string) {
        this._level = level;
        this._message = message;
    }

    private _level: LogLevel;
    get level(): LogLevel {
        return this._level;
    }

    set level(value: LogLevel) {
        this._level = value;
    }

    private _message: string;
    get message(): string {
        return this._message;
    }

    set message(value: string) {
        this._message = value;
    }
}