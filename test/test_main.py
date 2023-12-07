import re
import os
import pwd
import subprocess
from time import sleep
from functools import reduce

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
    try:
        os.remove("/tmp/taskmaster.sock")
        os.remove("/tmp/taskmaster.log")
    except:
        pass


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


def overwrite(dst, src):
    with open(dst, 'w') as fdst:
        with open(src, 'r') as fsrc:
            fdst.write(fsrc.read())


@pytest.fixture
def varying_text(request):
    (filename, origin, modified) = request.param

    overwrite(filename, origin)
    os.chmod(filename, 0o666)

    def change():
        overwrite(filename, modified)
    yield change


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
    assert re.match(
        r'tailf:0\s+Stopped\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d', output)


@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_stop_noexist(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # call stop (should not crash)
    output = get_ctl_result(tm, 'stop noexist')
    print(output)


def is_tmd(proc):
    try:
        cmd = proc.cmdline()
        return (
            len(cmd) == 2
            and cmd[0] == TMD
            and cmd[1] == 'test/start_simple_success.ini'
        )
    except:
        return False


@pytest.mark.parametrize("tm", ["test/start_simple_success.ini"], indirect=True)
def test_shutdown(tm):
    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # shutdown
    output = get_ctl_result(tm, 'shutdown')
    assert output == 'taskmasterd: shutdown'

    sleep(1)
    assert (not reduce(lambda acc, cur: acc or is_tmd(
        cur), psutil.process_iter(), False))


@pytest.mark.parametrize("tm", ["test/restart_simple_success.ini"], indirect=True)
def test_restart_simple_success(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in not running state.
    output = get_ctl_result(tm, 'restart tailf:0')
    print(output)
    assert output == 'tailf:0: not running.\r\ntailf:0: started'

    # wait util state turned into success
    sleep(3)

    # # show status
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'tailf:0\s+Running\s+pid \d+, uptime 0:00:\d\d', output)


@pytest.mark.parametrize("tm", ["test/restart_simple_success2.ini"], indirect=True)
def test_restart_simple_success2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'restart tailf:0')
    print(output)
    assert output == 'tailf:0: stopping\r\ntailf:0: started'

    # wait util state turned into success
    sleep(3)

    # show status
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'tailf:0\s+Running\s+pid \d+, uptime 0:00:\d\d', output)


@pytest.mark.parametrize("tm", ["test/restart_simple_success.ini"], indirect=True)
def test_restart_noexist(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'restart noexist:0')
    print(output)
    assert output == 'noexist:0: no such process.\r\nnoexist:0: no such process.'

# Must change path in ini file to your project path.


@pytest.mark.parametrize("tm", ["test/conf_command.ini"], indirect=True)
def test_conf_command(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    get_ctl_result(tm, 'start conf_command:0')

    sleep(2)

    with open('/tmp/conf_command.log') as f:
        contents = f.read()
        assert contents == 'arg1 arg2\narg1 arg2 arg3 arg4\n4\n'


@pytest.mark.parametrize("tm", ["test/conf_numprocs.ini"], indirect=True)
def test_conf_numprocs(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    output = get_ctl_result(tm, 'start all')

    expected = [
        r'conf_numprocs:0: started',
        r'conf_numprocs:1: started',
        r'conf_numprocs:2: started',
    ]

    print(output)
    assert all(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_autostart.ini"], indirect=True)
def test_conf_autostart(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_autostart:0\s+Starting",
        r"conf_autostart:0\s+Running\s+pid \d+, uptime 0:00:\d\d"
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_exitcodes.ini"], indirect=True)
def test_conf_exitcodes(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    sleep(3)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_exitcodes:0\s+Exited\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_autorestart_unexpected.ini"], indirect=True)
def test_autorestart_unexpected(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    for _ in range(0, 5):
        sleep(1)
        output = get_ctl_result(tm, 'status')

        expected = [
            r"conf_autorestart_unexpected:0\s+Starting",
            r"conf_autorestart_unexpected:0\s+Exited\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d\s+unexpected",
            r"conf_autorestart_unexpected:0\s+Running\s+pid \d+, uptime 0:00:\d\d"
        ]

        print(output)
        assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_autorestart_always.ini"], indirect=True)
def test_autorestart_always(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    for _ in range(0, 5):
        sleep(1)
        output = get_ctl_result(tm, 'status')

        expected = [
            r"conf_autorestart_always:0\s+Starting",
            r"conf_autorestart_always:0\s+Exited\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d",
            r"conf_autorestart_always:0\s+Running\s+pid \d+, uptime 0:00:\d\d"
        ]

        print(output)
        assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_autorestart_never.ini"], indirect=True)
def test_conf_autorestart_never(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    sleep(3)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_autorestart_never:0\s+Exited\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d\s+unexpected",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_startsecs.ini"], indirect=True)
def test_conf_startsecs(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    sleep(1)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs:0\s+Starting",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))

    sleep(1)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs:0\s+Starting",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))

    sleep(3)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs:0\s+Running\s+pid \d+, uptime 0:00:\d\d"
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_startsecs2.ini"], indirect=True)
def test_conf_startsecs2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    sleep(1)

    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs2:0\s+Starting",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))

    sleep(1)

    # input status to tmctl
    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs2:0\s+Starting",
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))

    sleep(2)

    # input status to tmctl
    output = get_ctl_result(tm, 'status')

    expected = [
        r"conf_startsecs2:0\s+Starting",
        r'conf_startsecs2:0\s+Fatal\s+Exited too quickly.',
        r'conf_startsecs2:0\s+Backoff\s+Exited too quickly.',
    ]

    print(output)
    assert any(map(lambda x: re.search(x, output) != None, expected))


@pytest.mark.parametrize("varying_text", [("test/update_simple.ini", "test/update_simple_origin.ini", "test/update_simple_modified.ini")], indirect=True)
@pytest.mark.parametrize("tm", ["test/update_simple.ini"], indirect=True)
def test_update(varying_text, tm):
    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # test status
    output = get_ctl_result(tm, 'status')
    print(output)
    # every process is not started yet
    assert re.match(r"a:0\s+Stopped\s+Not started", output)

    # change configure file
    varying_text()
    output = get_ctl_result(tm, 'update')
    assert 'configuration: updated' == output
    print(output)

    # test status
    output = get_ctl_result(tm, 'status')
    print(output)
    # every process is not started yet
    assert re.match(r"b:0\s+Stopped\s+Not started", output)


@pytest.mark.parametrize("tm", ["test/start_retries1.ini"], indirect=True)
def test_start_retries1(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'start dies:0')
    print(output)
    assert output == 'dies:0: started'

    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(
        r'dies:0\s+(Starting|Backoff|Fatal\s+Exited too quickly.)', output)

    sleep(2)
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'dies:0\s+Fatal\s+Exited too quickly.', output)


@pytest.mark.parametrize("tm", ["test/start_retries2.ini"], indirect=True)
def test_start_retries2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'start dies:0')
    print(output)
    assert output == 'dies:0: started'

    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(
        r'dies:0\s+(Starting|Backoff\s+Exited too quickly.)', output)

    sleep(3)
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'dies:0\s+Fatal\s+Exited too quickly.', output)


@pytest.mark.parametrize("tm", ["test/stop_signal1.ini"], indirect=True)
def test_stop_signal1(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    output = get_ctl_result(tm, 'start sig_echo:0')
    print(output)
    assert output == 'sig_echo:0: started'

    sleep(2)
    output = get_ctl_result(tm, 'stop sig_echo:0')
    print(output)
    assert output == 'sig_echo:0: stopping'

    sleep(1)
    with open('/tmp/sig_echo.log') as f:
        assert f.read() == 'INT\n'


@pytest.mark.parametrize("tm", ["test/stop_signal2.ini"], indirect=True)
def test_stop_signal2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    output = get_ctl_result(tm, 'start sig_echo:0')
    print(output)
    assert output == 'sig_echo:0: started'

    sleep(2)
    output = get_ctl_result(tm, 'stop sig_echo:0')
    print(output)
    assert output == 'sig_echo:0: stopping'

    sleep(2)
    with open('/tmp/sig_echo.log') as f:
        assert f.read() == 'TERM\n'


@pytest.mark.parametrize("tm", ["test/stopwaitsecs1.ini"], indirect=True)
def test_stopwaitsecs1(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'start ign_term:0')
    print(output)
    assert output == 'ign_term:0: started'

    sleep(2)
    output = get_ctl_result(tm, 'stop ign_term:0')
    print(output)
    assert output == 'ign_term:0: stopping'

    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'ign_term:0\s+Stopping', output)

    sleep(3)
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(
        r'ign_term:0\s+Stopped\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d', output)


@pytest.mark.parametrize("tm", ["test/stopwaitsecs2.ini"], indirect=True)
def test_stopwaitsecs2(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # restart process in running state
    output = get_ctl_result(tm, 'start ign_term:0')
    print(output)
    assert output == 'ign_term:0: started'

    sleep(2)
    output = get_ctl_result(tm, 'stop ign_term:0')
    print(output)
    assert output == 'ign_term:0: stopping'

    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(r'ign_term:0\s+Stopping', output)

    sleep(4)
    output = get_ctl_result(tm, 'status')
    print(output)
    assert re.match(
        r'ign_term:0\s+Stopped\s+\d\d\d\d-\d\d-\d\d \d\d:\d\d:\d\d.\d\d\d', output)


@pytest.mark.parametrize("tm", ["test/conf_directory.ini"], indirect=True)
def test_conf_directory(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # start process
    get_ctl_result(tm, 'start conf_directory:0')

    sleep(1)

    # check contents in log file
    with open('/tmp/conf_directory.log') as f:
        contents = f.read()
        print(contents)
        assert contents == '/tmp\n'


@pytest.mark.parametrize("tm", ["test/conf_environment.ini"], indirect=True)
def test_conf_env(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # start process
    get_ctl_result(tm, 'start conf_environment:0')

    sleep(1)

    with open('/tmp/conf_environment.log') as f:
        contents = f.read()

        expected = [
            r'A=1',
            r'B=2',
        ]
        print(contents)
        assert all(map(lambda x: re.search(x, contents) != None, expected))


@pytest.mark.parametrize("tm", ["test/conf_user.ini"], indirect=True)
def test_conf_user(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # start process
    get_ctl_result(tm, 'start conf_user:0')

    sleep(2)

    with open('/tmp/conf_user.log') as f:
        contents = f.read()

        print(contents)

        assert f'{os.geteuid()}\n' == contents

@pytest.mark.parametrize("tm", ["test/stderr_logfile.ini"], indirect=True)
def test_stderr_logfile(tm):

    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # start process
    get_ctl_result(tm, 'start eecho:0')

    sleep(2)

    with open('/tmp/eecho.err') as f:
        contents = f.read()
        print(contents)
        assert contents == 'Hello\n'

@pytest.mark.parametrize("tm", ["test/umask.ini"], indirect=True)
def test_umask(tm):    
    # ignore strings before first prompt
    tm.expect(r".*taskmaster> ")

    # start process
    get_ctl_result(tm, 'start touch:0')

    sleep(2)

    stat_result = os.stat('/tmp/somefile.txt')

    print(oct(stat_result.st_mode & 0o777))
    assert (stat_result.st_mode & 0o777) == 0o066

