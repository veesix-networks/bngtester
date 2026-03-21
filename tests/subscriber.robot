# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Settings ***
Library             OperatingSystem
Library             String

*** Keywords ***
Check Interface Exists
    [Arguments]    ${container}    ${iface}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} ip link show ${iface}
    Should Be Equal As Integers    ${rc}    0    Interface ${iface} not found

Check Interface Has IPv4
    [Arguments]    ${container}    ${iface}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} ip -4 addr show ${iface}
    Should Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    inet    No IPv4 address on ${iface}
    Should Not Contain    ${output}    169.254    Got link-local address, DHCP failed

Check Container Exited With Error
    [Arguments]    ${container}    ${expected_message}
    ${rc}    ${exit_code} =    Run And Return Rc And Output
    ...    sudo docker inspect -f '{{.State.ExitCode}}' ${container}
    Should Be Equal As Integers    ${rc}    0
    Should Not Be Equal As Strings    ${exit_code.strip()}    0    Container exited with 0, expected error
    ${rc}    ${logs} =    Run And Return Rc And Output
    ...    sudo docker logs ${container} 2>&1
    Log    ${logs}
    Should Contain    ${logs}    ${expected_message}

Check Container Log Contains
    [Arguments]    ${container}    ${expected}
    ${rc}    ${logs} =    Run And Return Rc And Output
    ...    sudo docker logs ${container} 2>&1
    Log    ${logs}
    Should Contain    ${logs}    ${expected}

Check Container Exit Code
    [Arguments]    ${container}    ${expected_code}
    ${rc}    ${exit_code} =    Run And Return Rc And Output
    ...    sudo docker inspect -f '{{.State.ExitCode}}' ${container}
    Should Be Equal As Integers    ${rc}    0
    Should Be Equal As Strings    ${exit_code.strip()}    ${expected_code}

Ping From Container
    [Arguments]    ${container}    ${target}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} ping -c 3 -W 2 ${target}
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0    Cannot ping ${target}

Send Signal To Container
    [Arguments]    ${container}    ${signal}=TERM
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker kill --signal ${signal} ${container}
    Should Be Equal As Integers    ${rc}    0

Send Signal If Running
    [Arguments]    ${container}    ${signal}=TERM
    ${rc}    ${status} =    Run And Return Rc And Output
    ...    sudo docker inspect -f '{{.State.Running}}' ${container}
    IF    '${status.strip()}' == 'true'
        Run And Return Rc And Output    sudo docker kill --signal ${signal} ${container}
    END

Wait For Container Exit
    [Arguments]    ${container}    ${timeout}=30
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    timeout ${timeout} sudo docker wait ${container}
    Log    Container ${container} exited with code: ${output}
    RETURN    ${output.strip()}

Wait For Interface In Container
    [Arguments]    ${container}    ${iface}    ${retries}=12    ${interval}=5s
    Wait Until Keyword Succeeds    ${retries} x    ${interval}
    ...    Check Interface Exists    ${container}    ${iface}

Wait Until Container Logs Contain
    [Arguments]    ${container}    ${expected}    ${retries}=12    ${interval}=5s
    Wait Until Keyword Succeeds    ${retries} x    ${interval}
    ...    Check Container Log Contains    ${container}    ${expected}

Run Subscriber Detached
    [Arguments]    ${name}    ${image}    ${env_args}    ${network}    ${caps}=--cap-add NET_ADMIN
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker run -d --name ${name} --network ${network} ${caps} ${env_args} ${image}
    Should Be Equal As Integers    ${rc}    0    Failed to start subscriber ${name}
    RETURN    ${output.strip()}
