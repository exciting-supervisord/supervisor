import re
import os
import pwd
import subprocess
from functools import reduce

import psutil
import pytest
import pexpect

TIMEOUT = 2

TEST_USER = "test"
TMCTL = "target/debug/tmctl"
TMD = "target/debug/tmd"

def is_euid(uid):
    def inner(proc):
        return proc.uids().effective == uid

    return inner


def is_root_or_exit():
    uid = os.getuid()
    if uid != 0:
        print("this script should run as root")
        exit(1)


def cleanup():
    # recover euid
    os.seteuid(0)

    # KILL every process which euid == TEST_USER
    uid = pwd.getpwnam(TEST_USER).pw_uid

    for proc in filter(is_euid(uid), psutil.process_iter()):
        proc.kill()

    # remove temp files
    os.remove("/tmp/taskmaster.sock")
    os.remove("/tmp/taskmaster.log")


def prerun(conf):
    # set euid to TEST_USER
    os.seteuid(pwd.getpwnam(TEST_USER).pw_uid)
    subprocess.run([TMD, conf])


@pytest.fixture
def tm(request):
    is_root_or_exit()
    prerun(request.param)
    yield pexpect.spawn(TMCTL, [request.param], timeout=TIMEOUT)
    cleanup()


@pytest.mark.parametrize("tm", ["test/status_before_begin.ini"], indirect=True)
def test_status_not_begin(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # test status
    tm.sendline("status")
    tm.expect(r"status\r\n(.*)\r\ntaskmaster> ")
    output = tm.match.group(1).decode("utf-8").strip()

    # every process is not started yet
    assert re.match(r"a:0\s+Stopped\s+Not started", output)


@pytest.mark.parametrize("tm", ["test/status_before_begin2.ini"], indirect=True)
def test_status_not_begin(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # test status
    tm.sendline("status")
    tm.expect(r"status\r\n(.*)\r\ntaskmaster> ")
    output = tm.match.group(1).decode("utf-8").strip()

    # every process is not started yet
    expected = [
        'a:0\s+Stopped\s+Not started',
        'b:0\s+Stopped\s+Not started',
        'c:0\s+Stopped\s+Not started',
        'd:0\s+Stopped\s+Not started'
    ]

    map(lambda x: re.search(x, output))

    assert re.search(r"a:0\s+Stopped\s+Not started", output)
    assert re.match(r"a:0\s+Stopped\s+Not started", output)
