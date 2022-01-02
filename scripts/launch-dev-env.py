# GNU AGPL v3 License

import boto3
import os
import subprocess as sp
import tempfile
import time

from botocore.exceptions import ClientError

BUCKET_NAME = "notgull"

def log(s):
    print(f" - {s}")

# is the file in this directory?
def dirHasFile(dir, fname):
    return any(d for d in os.listdir(dir) if fname in d)

# Look for a file.
# 
# This tries to find a specific file/path/etc that may be in an
# associated directory. It checks these folders, if they exist:
# 
# - Current dir
# - Parent dir
# - Current dir + proot
# - Parent dir + proot
# 
# Returns the directory that contains the path segment specified,
# or raises an error if it could not be found. 
def lookForFile(proot, fname):
    tries = [
        ".",
        proot,
        "..",
        os.path.join("..", proot)
    ]

    for try_ in tries:
        if os.path.exists(try_):
            if dirHasFile(try_, fname):
                return try_
    
    raise Exception(f"Could not find `{fname}`")

# Add a new client to the list.
def addClient(clientId, clientSecret, redirectUri):
    dyml = "docker-compose.yml"
    bpath = lookForFile("scripts", dyml)

    args = [
        "docker-compose",
        "exec",
        "hydra",
        "hydra",
        "clients",
        "create",
        "--endpoint",
        "http://127.0.0.1:4445/",
        "--id",
        clientId,
        "--secret",
        clientSecret,
        "--grant-types",
        "authorization_code,refresh_token",
        "--response-types",
        "code,id_token",
        "--scope",
        "openid,offline",
        "--callbacks",
        redirectUri
    ]

    log("Running client creater")
    sp.run(args, cwd=bpath)

# Launches docker-compose, bringing up localstack
# 
# Returns the docker-compose subprocess.
def dockerCompose(dir):
    dyml = "docker-compose.yml"
    bpath = lookForFile("scripts", dyml)

    # set DATA_DIR to our dir here
    dc_env = os.environ.copy()
    dc_env["DATA_DIR"] = dir

    # launch the docker-compose process
    p = sp.Popen(["docker-compose", "up"], cwd=bpath, env=dc_env, stdout=sp.PIPE)
    return p

sum_str = ""

# Stall until the given subprocess outputs the given byte sequence.
def waitForBytes(p, bytes):
    global sum_str
    for line in iter(p.stdout.readline, b''):
        sline = line.decode("utf-8")
        print(sline, end = "")
        sum_str += sline
        if bytes in sum_str:
            break

# Connect to the new localstack instance using Boto3
def connectS3():
    session = boto3.Session()
    s3_client = session.client(
        service_name="s3",
        aws_access_key_id="test",
        aws_secret_access_key="foobar",
        endpoint_url="http://localhost:4566"
    )
    return s3_client

# use an S3 client to upload the contents of the "public" directory
def uploadPublic(s3_client):
    public = "public"
    bpath = lookForFile("frontend", public)
    public = os.path.join(bpath, public)

    # create a bucket named "notgull"
    s3_client.create_bucket(
        Bucket=BUCKET_NAME, 
    )

    # walk over the "public" directory, and upload every file we can find
    for root, _, files in os.walk(public, topdown=False):
        # the absolute directory, and the root that we upload to AWS
        if not "public" in root:
            continue

        abs_root = os.path.abspath(root)
        s3_root = os.path.relpath(root, start=public)

        # iterate over files and get names to use
        for file in files:
            file_path = os.path.join(abs_root, file)
            object_path = os.path.join(s3_root, file)

            log(f"Uploading file {file_path}...")

            s3_client.upload_file(file_path, BUCKET_NAME, object_path)

            log(f"Uploaded file {file_path}")

# run npx gulp in the frontend dir
def runNpxGulp():
    bpath = lookForFile("frontend", "public")
    return sp.Popen(["npx", "gulp"], cwd=bpath)

def main():
    log("Running `npx gulp` to build frontend")
    gulp_process = runNpxGulp()

    try:
        with tempfile.TemporaryDirectory("ndntemp") as tempdir:
            dc_process = dockerCompose(tempdir)
            waitForBytes(dc_process, "Ready.")

            # make sure npx gulp is done before we upload public files
            gulp_process.wait()
            if gulp_process.returncode != 0:
                dc_process.terminate()
                raise Exception("Failed to run gulp")

            waitForBytes(dc_process, "msg=Setting up http server on :4445")
            time.sleep(1.5)

            addClient("ndndotnet", "ndndotsecret", "https://127.0.0.1:8199/callback")

            # now that we know we're ready, start uploading files
            s3_client = connectS3()
            uploadPublic(s3_client)

            try:
                input("Press enter to kill docker-compose")

            finally:
                dc_process.terminate()
                dc_process.wait()

                for line in iter(dc_process.stdout.readline, b''):
                    print(line.decode("utf-8"), end="")
    except PermissionError as e:
        # eat this error
        pass

if __name__ == "__main__":
    main()