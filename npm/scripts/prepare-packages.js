#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const tagPrefix = 'refs/tags/v';
const version = process.env.GITHUB_REF?.startsWith(tagPrefix)
  ? process.env.GITHUB_REF.slice(tagPrefix.length)
  : process.argv[2];
if (!version) {
  console.error('Error: Version not provided');
  console.error('Usage: node prepare-packages.js 0.1.0');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`Error: Invalid semantic version: ${version}`);
  process.exit(1);
}

console.log(`Preparing private SnoozeLine packages for version ${version}`);

// Define platform structures
const platforms = [
  'darwin-x64',
  'darwin-arm64',
  'linux-x64',
  'linux-x64-musl',
  'linux-arm64',
  'linux-arm64-musl',
  'win32-x64'
];

// Prepare platform packages
platforms.forEach(platform => {
  const sourceDir = path.join(__dirname, '..', 'platforms', platform);
  const targetDir = path.join(__dirname, '..', '..', 'npm-staging', platform);
  
  // Create directory
  fs.mkdirSync(targetDir, { recursive: true });
  
  // Read template package.json
  const templatePath = path.join(sourceDir, 'package.json');
  const packageJson = JSON.parse(fs.readFileSync(templatePath, 'utf8'));
  
  // Update version
  packageJson.version = version;
  packageJson.private = true;
  
  // Write to target directory
  fs.writeFileSync(
    path.join(targetDir, 'package.json'),
    JSON.stringify(packageJson, null, 2) + '\n'
  );
  
  console.log(`Prepared snoozeline-${platform} v${version}`);
});

// Prepare main package
const mainSource = path.join(__dirname, '..', 'main');
const mainTarget = path.join(__dirname, '..', '..', 'npm-staging', 'main');

// Copy main package files
fs.cpSync(mainSource, mainTarget, { recursive: true });

// Update main package.json
const mainPackageJsonPath = path.join(mainTarget, 'package.json');
const mainPackageJson = JSON.parse(fs.readFileSync(mainPackageJsonPath, 'utf8'));

mainPackageJson.version = version;
mainPackageJson.private = true;

// Update optionalDependencies versions
if (mainPackageJson.optionalDependencies) {
  Object.keys(mainPackageJson.optionalDependencies).forEach(dep => {
    if (dep.startsWith('snoozeline-')) {
      mainPackageJson.optionalDependencies[dep] = version;
    }
  });
}

fs.writeFileSync(
  mainPackageJsonPath,
  JSON.stringify(mainPackageJson, null, 2) + '\n'
);

console.log(`Prepared snoozeline v${version}`);
console.log('Private packages staged under npm-staging; publication is disabled.');
