# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Settings ***
Library             OperatingSystem
Library             String
Library             Process
Library             Collections

*** Variables ***
${CLAB_BIN}             sudo -E containerlab
${runtime}              docker
${OSVBNG_API_PORT}      8080
${HEALTH_RETRIES}       12
${HEALTH_INTERVAL}      10s
${VPPCTL_SOCK}          /run/osvbng/cli.sock
${SUBSCRIBER_IMAGE}     veesixnetworks/bngtester:alpine-latest

*** Keywords ***
Deploy Topology
    [Arguments]    ${topology_file}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    ${CLAB_BIN} deploy -t ${topology_file} --reconfigure
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0
    RETURN    ${output}

Destroy Topology
    [Arguments]    ${topology_file}
    Capture Container Logs    ${topology_file}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    ${CLAB_BIN} destroy -t ${topology_file} --cleanup
    Log    ${output}

Capture Container Logs
    [Arguments]    ${topology_file}
    ${rc}    ${containers} =    Run And Return Rc And Output
    ...    ${CLAB_BIN} inspect -t ${topology_file} --format json 2>/dev/null | python3 -c "import sys,json; cs=json.load(sys.stdin).get('containers',[]); print(' '.join(c['name'] for c in cs))" 2>/dev/null || true
    IF    '${containers}' != ''
        @{container_list} =    Split String    ${containers}
        FOR    ${container}    IN    @{container_list}
            ${rc}    ${logs} =    Run And Return Rc And Output
            ...    sudo docker logs ${container} 2>&1 | tail -200
            Log    Container logs for ${container}:\n${logs}    console=no
        END
    END

Get Container IPv4
    [Arguments]    ${container}
    ${rc}    ${ip} =    Run And Return Rc And Output
    ...    sudo docker inspect -f '{{range.NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${container}
    Should Be Equal As Integers    ${rc}    0
    Should Not Be Empty    ${ip}
    RETURN    ${ip}

Wait For osvbng Healthy
    [Arguments]    ${node}    ${lab_name}
    ${container} =    Set Variable    clab-${lab_name}-${node}
    Wait Until Keyword Succeeds    ${HEALTH_RETRIES} x    ${HEALTH_INTERVAL}
    ...    Check osvbng Started    ${container}

Check osvbng Started
    [Arguments]    ${container}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker logs ${container} 2>&1 | grep -q "osvbng started successfully"
    Should Be Equal As Integers    ${rc}    0    osvbng has not fully started yet

Execute VPP Command
    [Arguments]    ${container}    ${command}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} vppctl -s ${VPPCTL_SOCK} ${command}
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0
    RETURN    ${output}

Execute Vtysh On BNG
    [Arguments]    ${container}    ${command}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} ip netns exec dataplane vtysh -c "${command}"
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0
    RETURN    ${output}

Execute Vtysh On Router
    [Arguments]    ${container}    ${command}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} vtysh -c "${command}"
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0
    RETURN    ${output}

Get osvbng API Response
    [Arguments]    ${container}    ${path}
    ${ip} =    Get Container IPv4    ${container}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    curl -sf http://${ip}:${OSVBNG_API_PORT}${path}
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0
    RETURN    ${output}

Verify OSPF Adjacency On Router
    [Arguments]    ${container}
    ${output} =    Execute Vtysh On Router    ${container}    show ip ospf neighbor
    Should Contain    ${output}    Full

Run Container
    [Arguments]    ${name}    ${image}    ${env}=    ${caps}=    ${network}=none    ${extra}=
    ${env_args} =    Set Variable If    '${env}' != ''    ${env}    ${EMPTY}
    ${cap_args} =    Set Variable If    '${caps}' != ''    ${caps}    ${EMPTY}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker run --rm --name ${name} --network ${network} ${cap_args} ${env_args} ${extra} ${image} 2>&1
    Log    ${output}
    RETURN    ${rc}    ${output}

Run Container Detached
    [Arguments]    ${name}    ${image}    ${env}=    ${caps}=    ${network}=none    ${extra}=
    ${env_args} =    Set Variable If    '${env}' != ''    ${env}    ${EMPTY}
    ${cap_args} =    Set Variable If    '${caps}' != ''    ${caps}    ${EMPTY}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker run -d --name ${name} --network ${network} ${cap_args} ${env_args} ${extra} ${image}
    Should Be Equal As Integers    ${rc}    0    Failed to start container ${name}
    RETURN    ${output.strip()}

Remove Container
    [Arguments]    ${name}
    Run And Return Rc And Output    sudo docker rm -f ${name}

Create Docker Network
    [Arguments]    ${name}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker network create ${name}
    RETURN    ${output.strip()}

Remove Docker Network
    [Arguments]    ${name}
    Run And Return Rc And Output    sudo docker network rm ${name}
