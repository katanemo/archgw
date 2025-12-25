## Setup Instructions(User): plano CLI

This guide will walk you through the steps to set up the plano cli on your local machine

### Step 1: Create a Python virtual environment

In the tools directory, create a Python virtual environment by running:

```bash
python -m venv venv
```

### Step 2: Activate the virtual environment
* On Linux/MacOS:

```bash
source venv/bin/activate
```

### Step 3: Run the build script
```bash
pip install planoai==0.4.0
```

## Uninstall Instructions: plano CLI
```bash
pip uninstall planoai
```

## Setup Instructions (Dev): plano CLI

This guide will walk you through the steps to set up the plano cli on your local machine when you want to develop the plano CLI

### Step 1: Create a Python virtual environment

In the tools directory, create a Python virtual environment by running:

```bash
python -m venv venv
```

### Step 2: Activate the virtual environment
* On Linux/MacOS:

```bash
source venv/bin/activate
```

### Step 3: Run the build script
```bash
poetry install
```

### Step 4: build Arch
```bash
planoai build
```

### Logs
`plano` command can also view logs from the gateway. Use following command to view logs,

```bash
planoai logs --follow
```

## Uninstall Instructions: plano CLI
```bash
pip uninstall planoai
