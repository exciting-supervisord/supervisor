trap 'echo INT && exit 0' SIGINT
trap 'echo TERM && exit 0' SIGTERM
trap 'echo QUIT && exit 0' SIGQUIT

while true; do
    sleep 1
done