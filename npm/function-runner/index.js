#!/usr/bin/env node

import * as os from "os";
import * as path from "path";
import * as fs from "fs";
import * as childProcess from "child_process";
import * as gzip from "zlib";
import * as stream from "stream";
import fetch from "node-fetch";

const FR_URL = "https://github.com/Shopify/function-runner/releases/";
const FR_VERSION = "3.1.0";

async function main() {
	if (!(await isFrAvailable()) || process.env.REFRESH_FR) {
		console.error("function-runner is not available locally.");
		await fs.promises.unlink(frBinaryPath()).catch(() => {});
		if (process.env.BUILD_FR) {
			console.error("Building function-runner from source...");
			await buildFr();
			console.error("Done.");
		} else {
			console.error("Downloading function-runner ...");
			await downloadFr();
			console.error("Done.");
		}
	}
	try {
		childProcess.execFileSync(frBinaryPath(), getArgs());
	} catch (e) {
		if (typeof e?.status === "number") return;
		console.error(e);
	}
}
main();

function cacheDir(...suffixes) {
	const cacheDir = path.join(os.homedir(), ".fr_cache", ...suffixes);
	fs.mkdirSync(cacheDir, { recursive: true });
	return cacheDir;
}

function frBinaryPath() {
	return path.join(cacheDir(), "function-runner");
}

async function isFrAvailable() {
	return fs.promises
		.stat(frBinaryPath())
		.then(() => true)
		.catch(() => false);
}

async function downloadFr() {
	const compressedStream = await new Promise(async (resolve) => {
		const resp = await fetch(binaryUrl());
		resolve(resp.body);
	});
	const gunzip = gzip.createGunzip();
	const output = fs.createWriteStream(frBinaryPath());

	await new Promise((resolve, reject) => {
		stream.pipeline(compressedStream, gunzip, output, (err, val) => {
			if (err) return reject(err);
			return resolve(val);
		});
	});

	await fs.promises.chmod(frBinaryPath(), 0o775);
}

function binaryUrl() {
	// https://github.com/Shopify/function-runner/releases/download/v3.1.0/function-runner-x86_64-linux-v3.1.0.gz
	return `${FR_URL}/download/v${FR_VERSION}/function-runner-${platarch()}-v${FR_VERSION}.gz`;
}

const SUPPORTED_TARGETS = [
	"arm-macos",
	"x64_64-macos",
	"x64_64-windows",
	"x64_64-linux",
];

function platarch() {
	let platform, arch;
	switch (process.platform.toLowerCase()) {
		case "darwin":
			platform = "macos";
			break;
		case "linux":
			platform = "linux";
			break;
		case "win32":
			platform = "windows";
			break;
		default:
			throw Error(`Unsupported platform ${process.platform}`);
	}
	switch (process.arch.toLowerCase()) {
		case "arm":
		case "arm64":
			arch = "arm";
			break;
		case "x64":
			arch = "x86_64";
			break;
		default:
			throw Error(`Unsupported architecture ${process.arch}`);
	}
	const result = `${arch}-${platform}`;
	if (!SUPPORTED_TARGETS.includes(result)) {
		throw Error(
			`Unsupported platform/architecture combination ${platform}/${arch}`
		);
	}
	return result;
}

function getArgs() {
	const args = process.argv.slice(2);
	// TODO: Check if this needs to be changed when javy is installed via `npm install`.
	return args;
}

async function buildFr() {
	const repoDir = cacheDir("build", "fr");
	try {
		console.log("Downloading function-runners's source code...");
		childProcess.execSync(
			`git clone https://github.com/shopify/function-runner ${repoDir}`
		);
		console.log("Building function-runner...");
		childProcess.execSync("cargo build --release", { cwd: repoDir });
	} catch (e) {
		console.error(e);
		console.error("");
		console.error("BUILDING FUNCTION-RUNNER FAILED");
		console.error("Please make sure you have Rust installed");
		console.error("See the javy README for more details.");
	}
	await fs.promises.rename(
		path.join(repoDir, "target", "release", "function-runner"),
		frBinaryPath()
	);
}
