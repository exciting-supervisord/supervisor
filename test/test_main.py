import re
import os
import pwd
import subprocess
from time import sleep

import psutil
import pytest
import pexpect

TIMEOUT = 10

TEST_USER = "test"
TMCTL = "target/debug/tmctl"
TMD = "target/debug/tmd"

def get_ctl_result(tm, command):
    tm.sendline(command)
    tm.expect(rf"{command}\r\n(.*)\r\ntaskmaster> ")
    return tm.match.group(1).decode("utf-8").strip()

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
    output = get_ctl_result(tm, 'status')
    print(output)
    # every process is not started yet
    assert re.match(r"a:0\s+Stopped\s+Not started", output)


@pytest.mark.parametrize("tm", ["test/status_before_begin2.ini"], indirect=True)
def test_status_not_begin2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # test status
    output = get_ctl_result(tm, 'status')

    # every process is not started yet
    expected = [
        r'a:0\s+Stopped\s+Not started',
        r'b:0\s+Stopped\s+Not started',
        r'c:0\s+Stopped\s+Not started',
        r'd:0\s+Stopped\s+Not started'
    ]

    print(output)
    assert all(map(lambda x: re.search(x, output) != None, expected))

@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_start_not_exist_in_configure(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # test start
    output = get_ctl_result(tm, 'start no_exist')

    # should not crash
    print(output)

@pytest.mark.parametrize("tm", ["test/start_simple_fail.ini"], indirect=True)
def test_start_fail(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")
    
    # start process
    output = get_ctl_result(tm, 'start dies:0')
    print(output)
    assert output == 'dies:0: started'
    
    # wait util every retry fails
    sleep(5)

    # show status
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'dies:0\s+Fatal\s+Exited too quickly.', output)

@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_start_success(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")
    
    # start process
    output = get_ctl_result(tm, 'start tailf:0')
    print(output)
    assert output == 'tailf:0: started'
    
    # wait util state turned into success
    sleep(3)

    # show status
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'tailf:0\s+Running\s+pid \d+, uptime 0:00:\d\d', output)

@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_stop_success(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")
    
    # start process
    output = get_ctl_result(tm, 'start tailf:0')
    print(output)
    assert output == 'tailf:0: started'
    
    # wait util state turned into success
    sleep(3)

    # show status
    output = get_ctl_result(tm, 'stop tailf:0')
    print(output)
    assert output == 'tailf:0: stopping'

    sleep(1)
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'tailf:0\s+Stopped\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d', output)


@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_stop_noexist(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # call stop (should not crash)
    output = get_ctl_result(tm, 'stop noexist')
    print(output)


# @pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
# def test_shutdown(tm):
#     # ignore strings before first prompt
#     tm.expect(r".*taskmaster> ")

#     # call stop (should not crash)
#     output = get_ctl_result(tm, 'shutdown')
#     assert output == 'taskmasterd: shutdown'

#     sleep(1)
#     for item in psutil.process_iter():
#         item.cmdline